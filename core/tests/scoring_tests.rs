use groundscape_core::{
    AREA_SIZE_MM, ComponentWeights, LayoutObjective, PlacementRole, Pose, ScoredItem, score_layout,
    score_layout_delta, world_footprint_center,
};

fn objective() -> LayoutObjective {
    LayoutObjective::default()
}

/// Alan merkezinde merkezlenmiş kare footprint.
fn central(area: f64) -> ScoredItem {
    let half = area / 2.0;
    ScoredItem {
        center: [AREA_SIZE_MM / 2.0, AREA_SIZE_MM / 2.0],
        area: half * half * 4.0,
        role: PlacementRole::Distributed,
    }
}

fn item_at(x: f64, y: f64, area: f64, role: PlacementRole) -> ScoredItem {
    ScoredItem {
        center: [x, y],
        area,
        role,
    }
}

#[test]
fn empty_layout_scores_zero() {
    assert_eq!(score_layout(&[], &objective()), 0.0);
}

#[test]
fn world_center_follows_rotation() {
    // Footprint merkezi canonical (100, 0); 90° dönüş onu (0, 100)'e taşır.
    let pose = Pose::new(1000.0, 2000.0, std::f64::consts::FRAC_PI_2);
    let center = world_footprint_center(pose, [100.0, 0.0]);
    assert!((center[0] - 1000.0).abs() < 1e-9, "{center:?}");
    assert!((center[1] - 2100.0).abs() < 1e-9, "{center:?}");
}

#[test]
fn anchor_inside_region_cheaper_than_outside() {
    // Merkezi bölge: ±0.25·5000 = ±1250 alan merkezinden; bölge içi anchor
    // katkısı sıfırdır (plan P2 — tek noktaya çekmez).
    let inside = item_at(2800.0, 2200.0, 1000.0, PlacementRole::Anchor);
    let outside = item_at(4200.0, 4200.0, 1000.0, PlacementRole::Anchor);
    let layout_in = [inside, central(500.0)];
    let layout_out = [outside, central(500.0)];
    assert!(score_layout(&layout_in, &objective()) < score_layout(&layout_out, &objective()));
}

#[test]
fn anchor_outside_region_is_penalized() {
    let inside = item_at(2200.0, 2500.0, 1000.0, PlacementRole::Anchor);
    let outside = item_at(4000.0, 2500.0, 1000.0, PlacementRole::Anchor);
    let items_in = [inside, central(500.0)];
    let items_out = [outside, central(500.0)];
    assert!(score_layout(&items_in, &objective()) < score_layout(&items_out, &objective()));
}

#[test]
fn peripheral_role_is_excluded_from_anchor_term() {
    // Aynı konum: Anchor cezalandırılır, Peripheral edilmez.
    let as_anchor = item_at(4500.0, 4500.0, 400.0, PlacementRole::Anchor);
    let as_peripheral = item_at(4500.0, 4500.0, 400.0, PlacementRole::Peripheral);
    assert!(
        score_layout(&[as_anchor], &objective()) > score_layout(&[as_peripheral], &objective())
    );
}

#[test]
fn four_corners_score_worse_than_balanced_center() {
    // Tasarım §9 "dört köşe": bir büyük + üç küçük ürün köşelere yayıldığında
    // dengeli merkezi yerleşimden kötü puan alır (plan P2 kapısı).
    // Büyük ürün auto rolle etkin anchor'dır.
    let corner_spread = [
        item_at(500.0, 500.0, 1_440_000.0, PlacementRole::Auto),
        item_at(4500.0, 500.0, 90_000.0, PlacementRole::Distributed),
        item_at(500.0, 4500.0, 90_000.0, PlacementRole::Distributed),
        item_at(4500.0, 4500.0, 90_000.0, PlacementRole::Distributed),
    ];
    let compact = [
        item_at(2500.0, 2500.0, 1_440_000.0, PlacementRole::Auto),
        item_at(2100.0, 2500.0, 90_000.0, PlacementRole::Distributed),
        item_at(2900.0, 2500.0, 90_000.0, PlacementRole::Distributed),
        item_at(2500.0, 2100.0, 90_000.0, PlacementRole::Distributed),
    ];
    let corners = score_layout(&corner_spread, &objective());
    let balanced = score_layout(&compact, &objective());
    assert!(corners > balanced, "corner={corners} balanced={balanced}");
}

#[test]
fn spacing_penalizes_closeness_not_distance() {
    // Çiftler alan merkezine göre simetrik: balance ve anchor katkısı 0;
    // tek fark spacing terimidir.
    let target = objective().spacing_target_ratio * AREA_SIZE_MM;
    let close_pair = [
        item_at(
            2500.0 - target / 8.0,
            2500.0,
            100.0,
            PlacementRole::Distributed,
        ),
        item_at(
            2500.0 + target / 8.0,
            2500.0,
            100.0,
            PlacementRole::Distributed,
        ),
    ];
    let at_target_pair = [
        item_at(
            2500.0 - target / 2.0,
            2500.0,
            100.0,
            PlacementRole::Distributed,
        ),
        item_at(
            2500.0 + target / 2.0,
            2500.0,
            100.0,
            PlacementRole::Distributed,
        ),
    ];
    let far_pair = [
        item_at(1250.0, 2500.0, 100.0, PlacementRole::Distributed),
        item_at(3750.0, 2500.0, 100.0, PlacementRole::Distributed),
    ];
    let close = score_layout(&close_pair, &objective());
    let at_target = score_layout(&at_target_pair, &objective());
    let far = score_layout(&far_pair, &objective());
    assert!(close > at_target, "close={close} at_target={at_target}");
    // Sınırsız uzaklaşma cezalandırılmaz.
    assert!(far <= at_target + 1e-9, "far={far} at_target={at_target}");
}

#[test]
fn auto_role_treated_as_anchor_only_when_uniquely_largest() {
    // Benzersiz en büyük → anchor; benzer boyut (eşitlik) sahte ana ürün
    // seçmez (plan P2). Açık rol geometri çıkarımı yapmaz (plan §2.2).
    let big = Pose::new(4800.0, 4800.0, 0.0);
    let big_item = ScoredItem::new(big, [0.0, 0.0], 900.0, PlacementRole::Auto, 400.0);
    let tied_item = ScoredItem::new(big, [0.0, 0.0], 900.0, PlacementRole::Auto, 900.0);
    let small_item = ScoredItem::new(big, [0.0, 0.0], 100.0, PlacementRole::Auto, 900.0);
    assert_eq!(big_item.role, PlacementRole::Anchor);
    assert_eq!(tied_item.role, PlacementRole::Auto);
    assert_eq!(small_item.role, PlacementRole::Auto);

    // Açık rol geometri çıkarımı yapmaz (plan §2.2).
    let explicit = ScoredItem::new(big, [0.0, 0.0], 900.0, PlacementRole::Peripheral, 0.0);
    assert_eq!(explicit.role, PlacementRole::Peripheral);
}

#[test]
fn scoring_is_deterministic() {
    let items = [
        item_at(1200.0, 300.0, 400.0, PlacementRole::Anchor),
        item_at(3300.0, 4100.0, 200.0, PlacementRole::Distributed),
        item_at(4800.0, 900.0, 150.0, PlacementRole::Peripheral),
    ];
    assert_eq!(
        score_layout(&items, &objective()),
        score_layout(&items, &objective())
    );
}

#[test]
fn delta_equals_total_score_difference() {
    let placed = [
        item_at(2500.0, 2500.0, 900.0, PlacementRole::Anchor),
        item_at(3600.0, 2500.0, 100.0, PlacementRole::Distributed),
    ];
    let candidate = item_at(1200.0, 4400.0, 100.0, PlacementRole::Distributed);
    let obj = objective();
    let delta = score_layout_delta(candidate, &placed, &obj);
    let mut with = placed.to_vec();
    with.push(candidate);
    let expected = score_layout(&with, &obj) - score_layout(&placed, &obj);
    assert!((delta - expected).abs() < 1e-9);
}

#[test]
fn orientation_weight_zero_keeps_component_inert() {
    let obj = objective();
    assert_eq!(obj.weights.orientation, 0.0);
    let items = [item_at(0.0, 0.0, 100.0, PlacementRole::Distributed)];
    let obj_oriented = LayoutObjective {
        weights: ComponentWeights {
            orientation: 5.0,
            ..ComponentWeights::default()
        },
        ..obj.clone()
    };
    // Bileşen her zaman 0 döndürür; ağırlık önemsizdir.
    assert_eq!(
        score_layout(&items, &obj_oriented),
        score_layout(&items, &obj)
    );
}
