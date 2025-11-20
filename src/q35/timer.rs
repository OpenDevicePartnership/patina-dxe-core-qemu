//! QEMU Q35 Timer Calibration
//!
//! This module provides functionality to calibrate the tick frequency on
//! QEMU Q35 platforms using the ACPI Power Management Timer (PM Timer).
//!
//! ## References
//!
//! - [ACPI PM Timer](https://uefi.org/specs/ACPI/6.5/04_ACPI_Hardware_Specification.html)
//! - [FADT Table Definition](https://uefi.org/htmlspecs/ACPI_Spec_6_4_html/05_ACPI_Software_Programming_Model/ACPI_Software_Programming_Model.html#fixed-acpi-description-table-fadt)
//!
//! ## License
//!
//! Copyright (C) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!

use core::arch::x86_64;

const DEFAULT_ACPI_TIMER_FREQUENCY: u64 = 3_579_545; // 3.579545 MHz

pub fn calibrate_tsc_frequency(pm_timer_port: u16) -> u64 {
    // If there is an issue with the timer calibration loop, avoid hanging forever.
    const MAX_WAIT_CYCLES: usize = 1_000_000;

    // Wait for a PM timer edge to avoid partial intervals.
    let mut start_pm = read_pm_timer(pm_timer_port);
    let mut next_pm;
    let mut calibration_cycles_left = MAX_WAIT_CYCLES;
    loop {
        next_pm = read_pm_timer(pm_timer_port);
        if next_pm != start_pm {
            break;
        }

        calibration_cycles_left -= 1;
        // Avoid an infinite hang by breaking after too many cycles.
        // This means timer calibration may not be fully accurate, but can still safely proceed.
        if calibration_cycles_left == 0 {
            log::warn!("PM timer calibration timeout waiting for edge");
            break;
        }
    }
    start_pm = next_pm;

    // Record starting TSC.
    let start_tsc = unsafe { x86_64::_rdtsc() };

    // Hz = ticks/second. Divided by 20 ~ ticks / 50 ms.
    const TARGET_INTERVAL_SIZE: u64 = 20;
    let target_ticks = (DEFAULT_ACPI_TIMER_FREQUENCY / TARGET_INTERVAL_SIZE) as u32;

    let mut end_pm;
    calibration_cycles_left = MAX_WAIT_CYCLES;
    loop {
        end_pm = read_pm_timer(pm_timer_port);
        let delta = end_pm.wrapping_sub(start_pm);
        if delta >= target_ticks {
            break;
        }
        calibration_cycles_left -= 1;
        // If the PM timer is malfunctioning or not supported, avoid an infinite hang by breaking after too many cycles.
        // In this case we cannot safely proceed as will cause a zero division error, so we return a default value.
        // This default value is not accurate, but allows the system to proceed and gather relative timings still.
        if calibration_cycles_left == 0 {
            log::warn!("PM timer calibration timeout waiting for target ticks");
            return DEFAULT_ACPI_TIMER_FREQUENCY;
        }
    }

    // Record ending TSC.
    let end_tsc = unsafe { x86_64::_rdtsc() };

    // Time elapsed based on PM timer ticks.
    let delta_pm = end_pm.wrapping_sub(start_pm) as u64;
    let delta_time_ns = (delta_pm * 1_000_000_000) / DEFAULT_ACPI_TIMER_FREQUENCY;

    // Rdtsc ticks.
    let delta_tsc = end_tsc - start_tsc;

    // Frequency = Rdstc ticks / elapsed time.
    let freq_hz = (delta_tsc * 1_000_000_000) / delta_time_ns;

    freq_hz
}

fn read_pm_timer(pm_timer_port: u16) -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!(
            "in eax, dx",
            in("dx") pm_timer_port,
            out("eax") value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}
