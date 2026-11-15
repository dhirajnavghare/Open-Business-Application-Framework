use rust_decimal::Decimal;

#[test]
fn test_exact_gst_tax_split() {
    let taxable = Decimal::from(1000);
    let gst_rate = Decimal::from_str_exact("0.18").unwrap();

    let total_tax = taxable * gst_rate;
    assert_eq!(total_tax, Decimal::from(180));

    let cgst = total_tax / Decimal::from(2);
    let sgst = total_tax / Decimal::from(2);

    assert_eq!(cgst, Decimal::from(90));
    assert_eq!(sgst, Decimal::from(90));
    assert_eq!(cgst + sgst, total_tax);
    assert_eq!(taxable + total_tax, Decimal::from(1180));
}

#[test]
fn test_split_bill_penny_reconciliation() {
    // 100 dollars split among 3 diners:
    // 100 / 3 = 33.3333333...
    // Party 1: 33.33
    // Party 2: 33.33
    // Party 3: 33.34 (absorbs remainder)
    // Sum must equal EXACTLY 100.00
    let total = Decimal::from(100);
    let num_parties = 3;
    let divisor = Decimal::from(num_parties as i64);

    let base_share = (total / divisor).round_dp(2);
    assert_eq!(base_share, Decimal::from_str_exact("33.33").unwrap());

    let mut shares = Vec::new();
    let mut allocated = Decimal::ZERO;

    for _ in 1..num_parties {
        shares.push(base_share);
        allocated += base_share;
    }

    let last_share = total - allocated;
    shares.push(last_share);

    assert_eq!(last_share, Decimal::from_str_exact("33.34").unwrap());
    let sum: Decimal = shares.iter().copied().sum();
    assert_eq!(sum, total);
}
