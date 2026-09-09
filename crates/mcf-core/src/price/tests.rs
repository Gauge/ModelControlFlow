use super::PricePerKwh;

#[test]
fn a_price_is_read_the_way_a_person_types_it() {
    assert_eq!(
        PricePerKwh::parse("0.28"),
        Some(PricePerKwh {
            millionths: 280_000
        })
    );
    assert_eq!(
        PricePerKwh::parse(".28"),
        Some(PricePerKwh {
            millionths: 280_000
        })
    );
    assert_eq!(
        PricePerKwh::parse("28"),
        Some(PricePerKwh {
            millionths: 28_000_000
        })
    );
    assert_eq!(
        PricePerKwh::parse(" 0.075 "),
        Some(PricePerKwh { millionths: 75_000 })
    );
    assert_eq!(
        PricePerKwh::parse("0.123456"),
        Some(PricePerKwh {
            millionths: 123_456
        })
    );
}

#[test]
fn a_price_that_is_not_one_is_refused_rather_than_rounded() {
    for held in ["", "  ", "free", "0.1234567", "1.2.3", "-1", "0,28", "1e3"] {
        assert_eq!(PricePerKwh::parse(held), None, "{held:?} parsed as a price");
    }
}
