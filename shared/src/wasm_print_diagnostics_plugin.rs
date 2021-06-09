//use super::{Diagnostic, DiagnosticId, Diagnostics};
//use bevy_app::prelude::*;
//use bevy_core::{Time, Timer};
//use bevy_ecs::{IntoSystem, Res, ResMut};
//use bevy_utils::Duration;

use bevy::{
    app::prelude::*,
    core::{Time, Timer},
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics},
    ecs::prelude::*,
    //ecs::{IntoSystem, Res, ResMut},
    utils::Duration,
};

macro_rules! console_log {
    ($($t:tt)*) => (web_sys::console::log_1(&format_args!($($t)*).to_string().into()))
}

/// An App Plugin that prints diagnostics to the console
pub struct WasmPrintDiagnosticsPlugin {
    pub debug: bool,
    pub wait_duration: Duration,
    pub filter: Option<Vec<DiagnosticId>>,
}

/// State used by the [WasmPrintDiagnosticsPlugin]
pub struct WasmPrintDiagnosticsState {
    timer: Timer,
    filter: Option<Vec<DiagnosticId>>,
}

impl Default for WasmPrintDiagnosticsPlugin {
    fn default() -> Self {
        WasmPrintDiagnosticsPlugin {
            debug: false,
            wait_duration: Duration::from_secs(1),
            filter: None,
        }
    }
}

impl Plugin for WasmPrintDiagnosticsPlugin {
    fn build(&self, app: &mut bevy::app::AppBuilder) {
        app.insert_resource(WasmPrintDiagnosticsState {
            timer: Timer::new(self.wait_duration, true),
            filter: self.filter.clone(),
        });

        if self.debug {
            app.add_system_to_stage(
                CoreStage::PostUpdate,
                Self::print_diagnostics_debug_system.system(),
            );
        } else {
            app.add_system_to_stage(
                CoreStage::PostUpdate,
                Self::print_diagnostics_system.system(),
            );
        }
    }
}

impl WasmPrintDiagnosticsPlugin {
    pub fn filtered(filter: Vec<DiagnosticId>) -> Self {
        WasmPrintDiagnosticsPlugin {
            filter: Some(filter),
            ..Default::default()
        }
    }

    fn print_diagnostic(diagnostic: &Diagnostic) {
        if let Some(value) = diagnostic.value() {
            // TODO: buffer onto same line...
            console_log!("{:<65}: {:<10.6}", diagnostic.name, value);
            if let Some(average) = diagnostic.average() {
                // TODO: buffer onto same line...
                console_log!("  (avg {:.6})", average);
            }

            console_log!("\n");
        }
    }

    pub fn print_diagnostics_system(
        mut state: ResMut<WasmPrintDiagnosticsState>,
        time: Res<Time>,
        diagnostics: Res<Diagnostics>,
    ) {
        if state.timer.tick(time.delta()).finished() {
            console_log!("Diagnostics:");
            console_log!("{}", "-".repeat(93));
            if let Some(ref filter) = state.filter {
                for diagnostic in filter.iter().map(|id| diagnostics.get(*id).unwrap()) {
                    Self::print_diagnostic(diagnostic);
                }
            } else {
                for diagnostic in diagnostics.iter() {
                    Self::print_diagnostic(diagnostic);
                }
            }
        }
    }

    pub fn print_diagnostics_debug_system(
        mut state: ResMut<WasmPrintDiagnosticsState>,
        time: Res<Time>,
        diagnostics: Res<Diagnostics>,
    ) {
        if state.timer.tick(time.delta()).finished() {
            console_log!("Diagnostics (Debug):");
            console_log!("{}", "-".repeat(93));
            if let Some(ref filter) = state.filter {
                for diagnostic in filter.iter().map(|id| diagnostics.get(*id).unwrap()) {
                    console_log!("{:#?}\n", diagnostic);
                }
            } else {
                for diagnostic in diagnostics.iter() {
                    console_log!("{:#?}\n", diagnostic);
                }
            }
        }
    }
}
