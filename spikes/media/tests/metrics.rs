use media_spike::benchmark::{FrameSample, MetricsError, summarize};
fn sample(time: u64, captured: u64, rendered: u64) -> FrameSample {
    FrameSample {
        elapsed_ms: Some(time),
        captured: Some(captured),
        rendered: Some(rendered),
    }
}
#[test]
fn missing_frame_samples_fails_report() {
    assert_eq!(summarize(&[]), Err(MetricsError::MissingData));
    for incomplete in [
        FrameSample {
            elapsed_ms: None,
            captured: Some(30),
            rendered: Some(29),
        },
        FrameSample {
            elapsed_ms: Some(1000),
            captured: None,
            rendered: Some(29),
        },
        FrameSample {
            elapsed_ms: Some(1000),
            captured: Some(30),
            rendered: None,
        },
    ] {
        assert_eq!(
            summarize(&[sample(0, 0, 0), incomplete]),
            Err(MetricsError::MissingData)
        );
    }
}
#[test]
fn frame_rates_and_drops_use_the_measured_window() {
    let summary = summarize(&[sample(100, 3, 2), sample(1100, 33, 29)]).unwrap();
    assert_eq!(summary.duration_ms, 1000);
    assert_eq!(
        (
            summary.captured_frames,
            summary.rendered_frames,
            summary.dropped_frames
        ),
        (30, 27, 3)
    );
    assert_eq!((summary.capture_fps, summary.render_fps), (30., 27.));
    assert!(!summary.meets_minimum_duration);
}
#[test]
fn invalid_or_reset_counters_do_not_create_a_performance_claim() {
    for samples in [
        vec![sample(0, 0, 0)],
        vec![sample(100, 3, 2), sample(99, 4, 3)],
        vec![sample(0, 20, 10), sample(1000, 10, 20)],
        vec![sample(0, 0, 0), sample(1000, 10, 20)],
        vec![sample(0, 0, 0), sample(0, 30, 30)],
        vec![sample(0, 0, 0), sample(1000, 0, 0)],
    ] {
        assert_eq!(summarize(&samples), Err(MetricsError::InvalidSamples));
    }
}
#[test]
fn a_short_run_cannot_meet_the_ten_minute_gate() {
    assert!(
        !summarize(&[sample(0, 0, 0), sample(599_999, 18000, 17900)])
            .unwrap()
            .meets_minimum_duration
    );
    assert!(
        summarize(&[sample(0, 0, 0), sample(600_000, 18000, 17900)])
            .unwrap()
            .meets_minimum_duration
    );
}
