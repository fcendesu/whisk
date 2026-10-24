use media_spike::ui_metrics::{UiFrameMetrics, measurement_path};
use std::path::Path;
#[test]
fn preparation_metrics_count_cpu_copies_without_claiming_gpu_presentations() {
    let mut metrics = UiFrameMetrics::new("local_capture");
    metrics.prepared(8, 100, 1);
    metrics.prepared(16, 200, 2);
    metrics.tree_built();
    let report = metrics.finish(1_000);
    assert_eq!(report.cpu_prepared_frames, 2);
    assert_eq!(report.ui_bgra_bytes_copied, 24);
    assert_eq!(report.max_queue_depth, 2);
    assert_eq!(report.view_tree_builds, 1);
    assert_eq!(report.image_preparation_p95_us, Some(200));
    assert_eq!(report.observed_preparation_fps, Some(2.0));
    assert_eq!(report.gpu_presented_frames, None);
    assert!(!report.quality_certified);
}
#[test]
fn absent_frames_or_time_cannot_become_a_ui_performance_claim() {
    let empty = UiFrameMetrics::new("local_capture").finish(1_000);
    assert_eq!(empty.observed_preparation_fps, None);
    assert_eq!(empty.image_preparation_p95_us, None);
    let mut zero = UiFrameMetrics::new("local_capture");
    zero.prepared(8, 100, 1);
    assert_eq!(zero.finish(0).observed_preparation_fps, None);
}
#[test]
fn bundled_and_unbundled_apps_write_beside_the_app_without_using_launch_cwd() {
    assert_eq!(
        measurement_path(Path::new(
            "/repo/target/Whisk Media Lab.app/Contents/MacOS/app"
        )),
        Path::new("/repo/target/Whisk Media Lab-preview-metrics.json")
    );
    assert_eq!(
        measurement_path(Path::new("/repo/target/debug/app")),
        Path::new("/repo/target/debug/whisk-preview-metrics.json")
    );
}
