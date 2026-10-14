//! Scoring: calibration table, rounding and grades.

use stem::score::{grade, metric_score10, overall_score10};

/// Metric score for percentages of code in moderate / high / very high.
fn share(moderate: u64, high: u64, very_high: u64) -> (u32, &'static str) {
    let low = 100 - moderate - high - very_high;
    let score = metric_score10([low, moderate, high, very_high]);
    (score, grade(score))
}

#[test]
fn calibration_row_all_low_should_score_100() {
    assert_eq!(share(0, 0, 0), (1000, "A"));
}

#[test]
fn calibration_row_sig_top_grade_limit_should_score_87_5() {
    assert_eq!(share(25, 0, 0), (875, "A"));
}

#[test]
fn calibration_row_30_5_0_should_score_75() {
    assert_eq!(share(30, 5, 0), (750, "B"));
}

#[test]
fn calibration_row_40_10_0_should_score_60() {
    assert_eq!(share(40, 10, 0), (600, "C"));
}

#[test]
fn calibration_row_50_15_5_should_score_25() {
    assert_eq!(share(50, 15, 5), (250, "D"));
}

#[test]
fn calibration_row_all_moderate_should_score_50() {
    assert_eq!(share(100, 0, 0), (500, "D"));
}

#[test]
fn penalty_that_is_not_whole_tenths_should_round_the_score_down() {
    // 500 * 1 / 3 = 166.67 tenths of penalty; the ceiling makes it 167.
    assert_eq!(metric_score10([2, 1, 0, 0]), 833);
}

#[test]
fn average_ending_in_five_hundredths_should_round_half_up() {
    // (1000 + 1000 + 1000 + 998) / 4 = 999.5 tenths, so 100.0.
    assert_eq!(overall_score10([1000, 1000, 1000, 998]), 1000);
}

#[test]
fn average_below_half_should_round_down() {
    assert_eq!(overall_score10([1000, 1000, 1000, 997]), 999);
}

#[test]
fn score_should_never_go_below_zero() {
    assert_eq!(metric_score10([0, 0, 0, 10]), 0);
}

#[test]
fn grade_boundaries_should_match_the_table() {
    assert_eq!(
        [850, 849, 700, 699, 550, 549, 250, 249].map(grade),
        ["A", "B", "B", "C", "C", "D", "D", "E"]
    );
}
