



use core::arch::x86_64;

const DEFAULT_ACPI_TIMER_FREQUENCY: u64 = 3_579_545; // 3.579545 MHz

pub fn calibrate_tsc_frequency() -> u64 {
    // Wait for a PM timer edge to avoid partial intervals.
    let mut start_pm = read_pm_timer();
    let mut next_pm;
    loop {
        next_pm = read_pm_timer();
        if next_pm != start_pm {
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
    loop {
        end_pm = read_pm_timer();
        let delta = end_pm.wrapping_sub(start_pm);
        if delta >= target_ticks {
            break;
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

fn read_pm_timer() -> u32 {
    const PM_TIMER_PORT: u16 = 0x608; // Obtained from ACPI FADT X_PM_TIMER_BLOCK.
    let value: u32;
    unsafe {
        core::arch::asm!(
            "in eax, dx",
            in("dx") PM_TIMER_PORT,
            out("eax") value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}
