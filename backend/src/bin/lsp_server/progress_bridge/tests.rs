use super::*;

#[test]
fn stage_range_maps_percent_into_range() {
    let r = StageRange::new(10, 30);
    assert_eq!(r.map_percent_0_100(0), 10);
    assert_eq!(r.map_percent_0_100(50), 20);
    assert_eq!(r.map_percent_0_100(100), 30);
    assert_eq!(r.map_percent_0_100(1000), 30);
}

#[test]
fn stage_range_maps_current_total_into_range() {
    let r = StageRange::new(30, 90);
    assert_eq!(r.map_current_total(0, 0), 30);
    assert_eq!(r.map_current_total(0, 10), 30);
    assert_eq!(r.map_current_total(5, 10), 60);
    assert_eq!(r.map_current_total(10, 10), 90);
    assert_eq!(r.map_current_total(999, 10), 90);
}
