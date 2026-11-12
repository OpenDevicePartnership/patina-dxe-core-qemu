//! Patina Sample Components
//!
//! Adds every component from the `patina_samples` crate to a platform binary. The functions here are
//! no-ops unless the `samples` feature is enabled, so each binary can call them unconditionally.
//!
//! ## License
//!
//! Copyright (c) Microsoft Corporation.
//!
//! SPDX-License-Identifier: Apache-2.0
//!
use patina_dxe_core::{Add, Component, Config};

/// Adds the configuration consumed by the sample components.
#[cfg(feature = "samples")]
pub fn add_configs(add: &mut Add<Config>) {
    // `HelloStruct` is only dispatched once a `Config<i32>` (its "age") exists.
    add.config(42_i32);
}

/// Adds the configuration consumed by the sample components. This is a no-op without the `samples` feature.
#[cfg(not(feature = "samples"))]
pub fn add_configs(_add: &mut Add<Config>) {}

/// Adds every sample component from the `patina_samples` crate.
#[cfg(feature = "samples")]
pub fn add_components(add: &mut Add<Component>) {
    use patina_samples::{
        component::{
            hello_world::{GreetingsEnum, HelloStruct},
            uefi_services::{
                component_name_producer::ComponentNameProducerSample,
                configuration_table::{ConfigurationTableSample, DynamicConfigurationTableSample},
                driver_binding_producer::DriverBindingProducerSample,
                driver_connect::DriverConnectSample,
                driver_supported_efi_version_producer::DriverSupportedEfiVersionProducerSample,
                end_of_dxe_protocol_consumer::EndOfDxeProtocolConsumerSample,
                media_watcher::MediaWatcherSample,
                overview::UefiServicesSample,
                protocol_consumer::ProtocolConsumerSample,
                protocol_dependency::{OptionalProtocolDependencySample, ProtocolDependencySample},
                protocol_publisher::{
                    ProtocolConsumerSample as PublisherProtocolConsumerSample, ProtocolPublisherSample,
                },
                timers::TimerSample,
                tpl_critical_section::TplCriticalSectionSample,
            },
        },
        smbios_platform::SmbiosExampleComponent,
    };

    add.component(HelloStruct("Patina"));
    add.component(GreetingsEnum::Hello("Patina"));
    add.component(GreetingsEnum::Goodbye("Patina"));
    add.component(SmbiosExampleComponent::new());

    add.component(ComponentNameProducerSample::new());
    add.component(ConfigurationTableSample::new());
    add.component(DynamicConfigurationTableSample::new());
    add.component(DriverBindingProducerSample::new());
    add.component(DriverConnectSample::new());
    add.component(DriverSupportedEfiVersionProducerSample::new());
    add.component(EndOfDxeProtocolConsumerSample::new());
    add.component(MediaWatcherSample::new());
    add.component(UefiServicesSample::new());
    add.component(ProtocolConsumerSample::new());
    add.component(ProtocolDependencySample::new());
    add.component(OptionalProtocolDependencySample::new());
    add.component(ProtocolPublisherSample::new());
    add.component(PublisherProtocolConsumerSample::new());
    add.component(TimerSample::new());
    add.component(TplCriticalSectionSample::new());
}

/// Adds every sample component from the `patina_samples` crate. This is a no-op without the `samples` feature.
#[cfg(not(feature = "samples"))]
pub fn add_components(_add: &mut Add<Component>) {}
