use super::*;

#[test]
fn startup_progress_is_monotonic() {
    let coordinator = SystemCoordinator::new();

    coordinator.set_startup_progress(StartupProgressDto {
        percentage: 10.0,
        ..StartupProgressDto::default()
    });
    assert_eq!(coordinator.startup_progress().percentage, 10.0);

    coordinator.set_startup_progress(StartupProgressDto {
        percentage: 5.0,
        ..StartupProgressDto::default()
    });
    assert_eq!(coordinator.startup_progress().percentage, 10.0);
}

#[test]
fn startup_progress_clamps_100_before_done() {
    let coordinator = SystemCoordinator::new();

    coordinator.set_startup_progress(StartupProgressDto {
        percentage: 100.0,
        done: false,
        ..StartupProgressDto::default()
    });
    assert_eq!(coordinator.startup_progress().percentage, 99.0);

    coordinator.set_startup_progress(StartupProgressDto {
        percentage: 100.0,
        done: true,
        ..StartupProgressDto::default()
    });
    assert_eq!(coordinator.startup_progress().percentage, 100.0);
}
