use groundscape_core::{Engine, SearchConfig, Status, StepReport};

fn small_config() -> SearchConfig {
    SearchConfig {
        global_samples_per_item: 400,
        local_samples_per_item: 100,
        candidate_buffer_size: 6,
        max_restarts: 1,
        max_total_candidates: 5_000,
    }
}

#[test]
fn engine_steps_to_complete_within_budget() {
    let mut engine = Engine::empty();
    engine
        .add_product("a", &fixture_bytes("a", 1200.0))
        .unwrap();
    engine.add_product("b", &fixture_bytes("b", 900.0)).unwrap();

    engine.start_placement(small_config(), 11);
    let mut steps = 0;
    loop {
        let report = engine.step_placement(256);
        steps += 1;
        if report.done {
            let result = report.result.expect("final result");
            assert_eq!(result.status, Status::Complete, "{result:?}");
            assert_eq!(result.placements.len(), 2);
            break;
        }
        assert!(
            steps < 50,
            "steps must converge; total={}",
            report.total_candidates
        );
        assert!(report.total_candidates <= small_config().max_total_candidates);
    }
}

#[test]
fn budget_exhaustion_returns_final_result() {
    let mut engine = Engine::empty();
    // iki 3500²: toplam 24.5e6 ≤ 25e6 ama birlikte sığmaz
    engine
        .add_product("a", &fixture_bytes("a", 3500.0))
        .unwrap();
    engine
        .add_product("b", &fixture_bytes("b", 3500.0))
        .unwrap();

    engine.start_placement(small_config(), 3);
    let mut last: Option<StepReport> = None;
    for _ in 0..50 {
        let report = engine.step_placement(256);
        if report.done {
            last = Some(report);
            break;
        }
    }
    let final_report = last.expect("budget must exhaust");
    let result = final_report.result.expect("final result");
    assert_ne!(result.status, Status::Complete);
    assert_eq!(result.reason_code, "SEARCH_BUDGET_EXHAUSTED");
    assert!(final_report.total_candidates <= 5_000);
    // Sıra: büyükten küçüğe — ilki yerleşmiş
    assert_eq!(result.placements[0].product_id, "a");
}

#[test]
fn reset_clears_session() {
    let mut engine = Engine::empty();
    engine
        .add_product("a", &fixture_bytes("a", 1000.0))
        .unwrap();
    engine.start_placement(small_config(), 1);
    let _report = engine.step_placement(256);
    engine.reset_placement();

    let report = engine.step_placement(256);
    assert!(report.done && report.result.is_none() && report.placed_count == 0);
}

#[test]
fn zero_budget_returns_a_terminal_result() {
    let mut engine = Engine::empty();
    engine
        .add_product("a", &fixture_bytes("a", 1000.0))
        .unwrap();
    let mut config = small_config();
    config.max_total_candidates = 0;
    engine.start_placement(config, 1);

    let report = engine.step_placement(256);
    assert!(report.done);
    let result = report
        .result
        .expect("zero budget is a search result, not cancellation");
    assert_eq!(result.status, Status::NoSolutionFound);
    assert_eq!(result.reason_code, "SEARCH_BUDGET_EXHAUSTED");
}

#[test]
fn production_sampling_profile_progresses_across_products() {
    let mut engine = Engine::empty();
    for (id, size) in [("a", 1200.0), ("b", 900.0), ("c", 700.0)] {
        engine.add_product(id, &fixture_bytes(id, size)).unwrap();
    }
    let config = SearchConfig {
        max_total_candidates: 5_000,
        ..SearchConfig::default()
    };
    engine.start_placement(config, 42);

    loop {
        let report = engine.step_placement(256);
        assert!(report.ran_candidates <= 256);
        if report.done {
            assert_eq!(report.result.unwrap().status, Status::Complete);
            break;
        }
    }
}

#[test]
fn positive_slice_smaller_than_product_count_is_not_terminal() {
    let mut engine = Engine::empty();
    for (id, size) in [("a", 1200.0), ("b", 900.0), ("c", 700.0)] {
        engine.add_product(id, &fixture_bytes(id, size)).unwrap();
    }
    engine.start_placement(small_config(), 42);

    let report = engine.step_placement(1);
    assert_eq!(report.ran_candidates, 1);
    assert!(!report.done);
}

#[test]
fn one_candidate_slices_eventually_place_a_single_product() {
    let mut engine = Engine::empty();
    engine
        .add_product("a", &fixture_bytes("a", 1000.0))
        .unwrap();
    let mut config = small_config();
    config.max_total_candidates = 8;
    engine.start_placement(config, 42);

    let final_report = loop {
        let report = engine.step_placement(1);
        if report.done {
            break report;
        }
    };

    let result = final_report.result.expect("terminal result");
    assert_eq!(result.status, Status::Complete, "{result:?}");
    assert_eq!(result.placements.len(), 1);
}

fn run_with_slice(slice: usize, seed: u64) -> StepReport {
    let mut engine = Engine::empty();
    for (id, size) in [("a", 1200.0), ("b", 900.0), ("c", 700.0)] {
        engine.add_product(id, &fixture_bytes(id, size)).unwrap();
    }
    engine.start_placement(small_config(), seed);
    for _ in 0..small_config().max_total_candidates {
        let report = engine.step_placement(slice);
        if report.done {
            return report;
        }
    }
    panic!("search did not terminate");
}

#[test]
fn final_result_is_independent_of_worker_slice_size() {
    for seed in [1, 42, 99] {
        let one = run_with_slice(1, seed).result.expect("slice=1 result");
        let seven = run_with_slice(7, seed).result.expect("slice=7 result");
        let two_fifty_six = run_with_slice(256, seed).result.expect("slice=256 result");

        assert_eq!(one.status, seven.status);
        assert_eq!(one.placements, seven.placements);
        assert_eq!(seven.status, two_fifty_six.status);
        assert_eq!(seven.placements, two_fifty_six.placements);
    }
}

#[test]
fn empty_product_set_returns_controlled_invalid_result() {
    let mut engine = Engine::empty();
    let started = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        engine.start_placement(small_config(), 1);
    }));
    assert!(started.is_ok(), "empty input must not panic");

    let report = engine.step_placement(1);
    assert!(report.done);
    let result = report.result.expect("invalid input result");
    assert_eq!(result.status, Status::InvalidInput);
    assert_eq!(result.reason_code, "EMPTY_PRODUCT_SET");
}

#[test]
fn worker_dtos_use_typescript_names() {
    let status = serde_json::to_value(Status::Complete).unwrap();
    assert_eq!(status, "COMPLETE");

    let report = StepReport {
        ran_candidates: 1,
        done: false,
        placed_count: 2,
        total_candidates: 3,
        result: None,
    };
    let report = serde_json::to_value(report).unwrap();
    assert_eq!(report["ranCandidates"], 1);
    assert_eq!(report["placedCount"], 2);
    assert_eq!(report["totalCandidates"], 3);
}

fn fixture_bytes(_id: &str, size: f64) -> Vec<u8> {
    // Basit mm DXF: FOOTPRINT kare + daha büyük SAFETY_ZONE kare, ortak (0,0).
    let safety = size;
    let footprint = size * 0.6;
    let contour = |layer: &str, s: f64| {
        format!(
            "0\nLWPOLYLINE\n100\nAcDbEntity\n8\n{layer}\n100\nAcDbPolyline\n90\n4\n70\n1\n10\n0\n20\n0\n10\n{s}\n20\n0\n10\n{s}\n20\n{s}\n10\n0\n20\n{s}\n"
        )
    };
    format!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1015\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n{}{}\n0\nENDSEC\n0\nEOF\n",
        contour("FOOTPRINT", footprint),
        contour("SAFETY_ZONE", safety),
    )
    .into_bytes()
}
