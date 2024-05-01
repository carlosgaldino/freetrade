use std::{borrow::Cow, fmt::Write};

use time::OffsetDateTime;

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let input_file;
    let account_name;
    match args.next() {
        None => return Err(USAGE.into()),
        Some(s) => match s.as_str() {
            "-h" | "--help" => {
                eprintln!("{}", USAGE);
                return Ok(());
            }
            "-i" | "--input" => {
                input_file = args.next().ok_or(USAGE)?;
                account_name = expect_arg("-a", "--account", &mut args)?;
            }
            "-a" | "--account" => {
                account_name = args.next().ok_or(USAGE)?;
                input_file = expect_arg("-i", "--input", &mut args)?;
            }
            _ => return Err(USAGE.into()),
        },
    }
    println!(";; Imported on {}", time::OffsetDateTime::now_utc());
    println!(";; Account: {}", account_name);
    println!(";; Original file: {}\n", input_file);
    let mut reader = csv::Reader::from_path(input_file)?;
    for result in reader.deserialize() {
        let record: Record = result?;
        let formatted = record.format(&account_name)?;
        if !formatted.is_empty() {
            println!("{}", formatted);
        }
    }

    Ok(())
}

fn expect_arg(
    arg_short: &str,
    arg_long: &str,
    args: &mut impl Iterator<Item = String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let arg = args.next().ok_or(USAGE)?;
    let arg = match arg.as_str() {
        short if short == arg_short => args.next(),
        long if long == arg_long => args.next(),
        _ => return Err(USAGE.into()),
    };
    arg.ok_or_else(|| USAGE.into())
}

const USAGE: &str = r#"freetrade - A beancount importer for Freetrade

Usage:
  freetrade --input <FILE> --account <ACCOUNT>
  freetrade --help

Options:
  -i --input   Input file.
  -a --account Account name to use. In the output it will be "Assets:UK:Freetrade:ACCOUNT:SYMBOL".
  -h --help    Print this message.
"#;

#[derive(Debug, PartialEq, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum OrderType {
    Buy,
    Sell,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Type {
    Dividend,
    InterestFromCash,
    MonthlyStatement,
    Order,
    SippAnnualStatement,
    SippPresaleIllustration,
    TaxRelief,
    TopUp,
}

#[derive(Debug, serde::Deserialize)]
struct Record {
    #[serde(rename = "Type")]
    kind: Type,
    #[serde(
        rename = "Timestamp",
        deserialize_with = "time::serde::rfc3339::deserialize"
    )]
    timestamp: time::OffsetDateTime,
    #[serde(rename = "Total Amount")]
    total_amount: Option<f32>,
    #[serde(rename = "Price per Share in Account Currency")]
    price: Option<f32>,
    #[serde(rename = "Buy / Sell")]
    order_type: Option<OrderType>,
    #[serde(rename = "Ticker")]
    ticker: Option<String>,
    #[serde(rename = "Stamp Duty")]
    stamp_duty: Option<f32>,
    #[serde(rename = "Quantity")]
    quantity: Option<f32>,
    #[serde(rename = "FX Fee Amount")]
    fx_fee_amount: Option<f32>,
}

impl Record {
    fn format(&self, account: &str) -> Result<String, Box<dyn std::error::Error>> {
        match self.kind {
            Type::Order => self.format_order(account),
            Type::Dividend | Type::InterestFromCash | Type::TaxRelief | Type::TopUp => {
                self.format_credit(account)
            }
            Type::MonthlyStatement | Type::SippAnnualStatement | Type::SippPresaleIllustration => {
                // Nothing to do. These are always empty.
                Ok(String::new())
            }
        }
    }

    fn format_credit(&self, account: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut buf = String::new();
        self.write_timestamp(&mut buf)?;
        match &self.kind {
            Type::Dividend => buf.write_str(r#""Dividend""#)?,
            Type::InterestFromCash => buf.write_str(r#""Interest from cash""#)?,
            Type::TaxRelief => buf.write_str(r#""Tax Relief""#)?,
            Type::TopUp => buf.write_str(r#""Top Up""#)?,
            _type => panic!("format_credit: invalid record type: {:?}", _type),
        }
        buf.write_char('\n')?;
        buf.write_str("    Assets:UK:Freetrade:")?;
        buf.write_str(account)?;
        buf.write_str(":Checking ")?;
        buf.write_str(&self.total_amount.expect("missing total amount").to_string())?;
        buf.write_str(" GBP\n")?;

        // Income
        buf.write_str("    Income:UK:Freetrade:")?;
        buf.write_str(account)?;
        if matches!(self.kind, Type::Dividend) {
            buf.write_char(':')?;
            // Normalise ticker name
            buf.write_str(
                &self
                    .ticker
                    .as_ref()
                    .expect("dividend without a ticker")
                    .replace('.', ""),
            )?;
        }
        match self.kind {
            Type::Dividend => buf.write_str(":Dividend -")?,
            Type::InterestFromCash => buf.write_str(":Interest -")?,
            Type::TaxRelief => buf.write_str(":TaxRelief -")?,
            Type::TopUp => buf.write_str(":TopUp -")?,
            _ => unreachable!("should've panicked before"),
        }
        buf.write_str(&self.total_amount.expect("missing total amount").to_string())?;
        buf.write_str(" GBP\n")?;

        Ok(buf)
    }

    fn write_timestamp(&self, buf: &mut String) -> Result<(), Box<dyn std::error::Error>> {
        format_timestamp(buf, self.timestamp)?;
        if self.timestamp <= OffsetDateTime::now_utc() {
            buf.write_str(" * ")?;
        } else {
            buf.write_str(" ! ")?;
        }

        Ok(())
    }

    fn format_order(&self, account: &str) -> Result<String, Box<dyn std::error::Error>> {
        assert!(matches!(self.kind, Type::Order));

        let mut buf = String::new();
        self.write_timestamp(&mut buf)?;
        buf.write_char('"')?;
        if self.order_type.as_ref().expect("order without type") == &OrderType::Buy {
            buf.write_str("Buy ")?;
        } else {
            buf.write_str("Sell ")?;
        }
        let ticker = self.ticker.as_ref().expect("order without a ticker");
        buf.write_str(ticker)?;
        buf.write_char('"')?;
        buf.write_char('\n')?;

        // Assets
        buf.write_str("    Assets:UK:Freetrade:")?;
        buf.write_str(account)?;
        buf.write_char(':')?;
        buf.write_str(&ticker.replace('.', ""))?; // Normalise ticker name
        buf.write_char(' ')?;
        if self
            .order_type
            .as_ref()
            .map_or(false, |ot| *ot == OrderType::Sell)
        {
            buf.write_char('-')?;
        }
        buf.write_str(&self.quantity.expect("order without quantity").to_string())?;
        buf.write_char(' ')?;
        let normalised_ticker = format_ticker(ticker);
        buf.write_str(&normalised_ticker)?;
        buf.write_str(" {")?;
        buf.write_str(
            &self
                .price
                .expect("order without price per share")
                .to_string(),
        )?;
        buf.write_str(" GBP}\n")?;

        // FX Fee
        if let Some(fx_fee) = self.fx_fee_amount {
            buf.write_str("    Expenses:UK:Freetrade:")?;
            buf.write_str(account)?;
            buf.write_str(":Fees ")?;
            buf.write_str(&fx_fee.to_string())?;
            buf.write_str(" GBP")?;
            buf.write_char('\n')?;
        }

        // Stamp Duty
        if let Some(stamp_duty) = self.stamp_duty {
            if stamp_duty != 0.0 {
                buf.write_str("    Expenses:UK:Freetrade:")?;
                buf.write_str(account)?;
                buf.write_str(":StampDuty ")?;
                buf.write_str(&stamp_duty.to_string())?;
                buf.write_str(" GBP")?;
                buf.write_char('\n')?;
            }
        }

        // Total
        buf.write_str("    Assets:UK:Freetrade:")?;
        buf.write_str(account)?;
        buf.write_str(":Checking ")?;
        if self
            .order_type
            .as_ref()
            .map_or(false, |ot| *ot == OrderType::Buy)
        {
            buf.write_char('-')?;
        }
        buf.write_str(
            &self
                .total_amount
                .expect("order without total amount")
                .to_string(),
        )?;
        buf.write_str(" GBP")?;
        buf.write_char('\n')?;

        Ok(buf)
    }
}

fn format_ticker(ticker: &str) -> Cow<'_, str> {
    if ticker.len() == 1 {
        format!("{}{}", ticker, ticker).into()
    } else {
        ticker.into()
    }
}

fn format_timestamp(
    buf: &mut String,
    ts: OffsetDateTime,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = time::macros::format_description!("[year]-[month]-[day]");
    let ts = ts.format(format)?;
    buf.write_str(&ts)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use crate::{OrderType, Record, Type};

    #[test]
    fn test_format_order_buy() {
        let date = OffsetDateTime::parse(
            "2020-01-02T03:04:05Z",
            &time::format_description::well_known::Rfc3339,
        )
        .expect("valid date");
        let mut record = Record {
            kind: Type::Order,
            timestamp: date,
            total_amount: Some(90.15),
            price: Some(30.0),
            order_type: Some(OrderType::Buy),
            ticker: Some("FOO".into()),
            quantity: Some(3.0),
            fx_fee_amount: Some(0.15),
            stamp_duty: None,
        };
        let expected = r#"2020-01-02 * "Buy FOO"
    Assets:UK:Freetrade:SIPP:FOO 3 FOO {30 GBP}
    Expenses:UK:Freetrade:SIPP:Fees 0.15 GBP
    Assets:UK:Freetrade:SIPP:Checking -90.15 GBP
"#;
        let buf = record.format("SIPP").expect("valid record");
        assert_eq!(buf, expected);

        // Ticker with invalid characters for beancount.
        record.ticker = Some("BAR.Z".into());
        let expected = r#"2020-01-02 * "Buy BAR.Z"
    Assets:UK:Freetrade:SIPP:BARZ 3 BAR.Z {30 GBP}
    Expenses:UK:Freetrade:SIPP:Fees 0.15 GBP
    Assets:UK:Freetrade:SIPP:Checking -90.15 GBP
"#;
        let buf = record.format("SIPP").expect("valid record");
        assert_eq!(buf, expected);

        // Future date, no FX fees, and stamp duty.
        let date = OffsetDateTime::parse(
            "2050-01-02T03:04:05Z",
            &time::format_description::well_known::Rfc3339,
        )
        .expect("valid date");
        let record = Record {
            kind: Type::Order,
            timestamp: date,
            total_amount: Some(92.19),
            price: Some(30.0),
            order_type: Some(OrderType::Buy),
            ticker: Some("FOO".into()),
            quantity: Some(3.0),
            fx_fee_amount: None,
            stamp_duty: Some(2.19),
        };
        let expected = r#"2050-01-02 ! "Buy FOO"
    Assets:UK:Freetrade:SIPP:FOO 3 FOO {30 GBP}
    Expenses:UK:Freetrade:SIPP:StampDuty 2.19 GBP
    Assets:UK:Freetrade:SIPP:Checking -92.19 GBP
"#;
        let buf = record.format("SIPP").expect("valid record");
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_format_order_sell() {
        let date = OffsetDateTime::parse(
            "2020-01-02T03:04:05Z",
            &time::format_description::well_known::Rfc3339,
        )
        .expect("valid date");
        let record = Record {
            kind: Type::Order,
            timestamp: date,
            total_amount: Some(90.0),
            price: Some(30.0),
            order_type: Some(OrderType::Sell),
            ticker: Some("FOO".into()),
            quantity: Some(3.0),
            fx_fee_amount: None,
            stamp_duty: Some(0.0),
        };
        let expected = r#"2020-01-02 * "Sell FOO"
    Assets:UK:Freetrade:SIPP:FOO -3 FOO {30 GBP}
    Assets:UK:Freetrade:SIPP:Checking 90 GBP
"#;
        let buf = record.format("SIPP").expect("valid record");
        assert_eq!(buf, expected);

        // Single char ticker
        let record = Record {
            kind: Type::Order,
            timestamp: date,
            total_amount: Some(90.0),
            price: Some(30.0),
            order_type: Some(OrderType::Sell),
            ticker: Some("F".into()),
            quantity: Some(3.0),
            fx_fee_amount: None,
            stamp_duty: Some(0.0),
        };
        let expected = r#"2020-01-02 * "Sell F"
    Assets:UK:Freetrade:SIPP:F -3 FF {30 GBP}
    Assets:UK:Freetrade:SIPP:Checking 90 GBP
"#;
        let buf = record.format("SIPP").expect("valid record");
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_format_top_up() {
        let date = OffsetDateTime::parse(
            "2020-01-02T03:04:05Z",
            &time::format_description::well_known::Rfc3339,
        )
        .expect("valid date");
        let record = Record {
            kind: Type::TopUp,
            timestamp: date,
            total_amount: Some(25.5),
            price: None,
            order_type: None,
            ticker: None,
            quantity: None,
            fx_fee_amount: None,
            stamp_duty: None,
        };

        let expected = r#"2020-01-02 * "Top Up"
    Assets:UK:Freetrade:SIPP:Checking 25.5 GBP
    Income:UK:Freetrade:SIPP:TopUp -25.5 GBP
"#;
        let buf = record.format("SIPP").expect("valid record");
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_format_tax_relief() {
        let date = OffsetDateTime::parse(
            "2020-01-02T03:04:05Z",
            &time::format_description::well_known::Rfc3339,
        )
        .expect("valid date");
        let record = Record {
            kind: Type::TaxRelief,
            timestamp: date,
            total_amount: Some(35.5),
            price: None,
            order_type: None,
            ticker: None,
            quantity: None,
            fx_fee_amount: None,
            stamp_duty: None,
        };

        let expected = r#"2020-01-02 * "Tax Relief"
    Assets:UK:Freetrade:SIPP:Checking 35.5 GBP
    Income:UK:Freetrade:SIPP:TaxRelief -35.5 GBP
"#;
        let buf = record.format("SIPP").expect("valid record");
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_format_interest() {
        let date = OffsetDateTime::parse(
            "2020-01-02T03:04:05Z",
            &time::format_description::well_known::Rfc3339,
        )
        .expect("valid date");
        let record = Record {
            kind: Type::InterestFromCash,
            timestamp: date,
            total_amount: Some(5.5),
            price: None,
            order_type: None,
            ticker: None,
            quantity: None,
            fx_fee_amount: None,
            stamp_duty: None,
        };

        let expected = r#"2020-01-02 * "Interest from cash"
    Assets:UK:Freetrade:SIPP:Checking 5.5 GBP
    Income:UK:Freetrade:SIPP:Interest -5.5 GBP
"#;
        let buf = record.format("SIPP").expect("valid record");
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_format_dividend() {
        let date = OffsetDateTime::parse(
            "2020-01-02T03:04:05Z",
            &time::format_description::well_known::Rfc3339,
        )
        .expect("valid date");
        let record = Record {
            kind: Type::Dividend,
            timestamp: date,
            total_amount: Some(25.5),
            price: None,
            order_type: None,
            ticker: Some("ABC.V".into()),
            quantity: None,
            fx_fee_amount: None,
            stamp_duty: None,
        };

        let expected = r#"2020-01-02 * "Dividend"
    Assets:UK:Freetrade:SIPP:Checking 25.5 GBP
    Income:UK:Freetrade:SIPP:ABCV:Dividend -25.5 GBP
"#;
        let buf = record.format("SIPP").expect("valid record");
        assert_eq!(buf, expected);

        // Single char ticker
        let record = Record {
            kind: Type::Dividend,
            timestamp: date,
            total_amount: Some(25.5),
            price: None,
            order_type: None,
            ticker: Some("A".into()),
            quantity: None,
            fx_fee_amount: None,
            stamp_duty: None,
        };

        let expected = r#"2020-01-02 * "Dividend"
    Assets:UK:Freetrade:SIPP:Checking 25.5 GBP
    Income:UK:Freetrade:SIPP:A:Dividend -25.5 GBP
"#;
        let buf = record.format("SIPP").expect("valid record");
        assert_eq!(buf, expected);
    }
}
