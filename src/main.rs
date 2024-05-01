use std::fmt::Write;

use time::OffsetDateTime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let mut reader = csv::Reader::from_path(input_file)?;
    for result in reader.deserialize() {
        let record: Record = result?;
        println!("{}", record.format(&account_name)?);
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
    #[serde(rename = "Quantity")]
    quantity: Option<f32>,
    #[serde(rename = "Instrument Currency")]
    currency: Option<String>,
    #[serde(rename = "Total Shares Amount")]
    total_shares_amount: Option<f32>,
    #[serde(rename = "FX Rate")]
    fx_rate: Option<f32>,
    #[serde(rename = "Base FX Rate")]
    base_fx_rate: Option<f32>,
    #[serde(rename = "FX Fee Amount")]
    fx_fee_amount: Option<f32>,
    #[serde(rename = "Dividend Eligible Quantity")]
    dividend_eligible_quantity: Option<f32>,
    #[serde(rename = "Dividend Amount Per Share")]
    dividend_amount_per_share: Option<f32>,
    #[serde(rename = "Dividend Gross Distribution Amount")]
    dividend_gross_distribution_amount: Option<f32>,
    #[serde(rename = "Dividend Net Distribution Amount")]
    dividend_net_distribution_amount: Option<f32>,
    #[serde(rename = "Dividend Withheld Tax Amount")]
    dividend_withheld_tax_amount: Option<f32>,
}

impl Record {
    fn format(&self, account: &str) -> Result<String, Box<dyn std::error::Error>> {
        match self.kind {
            Type::Dividend => Ok(String::new()),
            Type::InterestFromCash => Ok(String::new()),
            Type::MonthlyStatement => Ok(String::new()),
            Type::Order => self.format_order(account),
            Type::SippAnnualStatement => Ok(String::new()),
            Type::SippPresaleIllustration => Ok(String::new()),
            Type::TaxRelief => Ok(String::new()),
            Type::TopUp => Ok(String::new()),
        }
    }

    fn format_order(&self, account: &str) -> Result<String, Box<dyn std::error::Error>> {
        assert!(matches!(self.kind, Type::Order));
        /*
         * 2023-07-02 * "Order BUY VISA"
         *     Assets:UK:Freetrade:ISA:VISA AMOUNT VISA {}
         */
        let mut buf = String::new();
        buf.write_char('\n')?;
        format_timestamp(&mut buf, self.timestamp)?;
        if self.timestamp <= OffsetDateTime::now_utc() {
            buf.write_str(" * ")?;
        } else {
            buf.write_str(" ! ")?;
        }
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
        buf.write_str(ticker)?;
        buf.write_char(' ')?;
        buf.write_str(&self.quantity.expect("order without quantity").to_string())?;
        buf.write_char(' ')?;
        buf.write_str(ticker)?;
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

        // Total
        buf.write_str("    Assets:UK:Freetrade:")?;
        buf.write_str(account)?;
        buf.write_str(":Checking -")?;
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
    #[test]
    fn test_format_order() {
        // TODO: Tickers with characters not allowed: needs replacement
        // Tickers without fx fees
        // Sell orders
    }
}
