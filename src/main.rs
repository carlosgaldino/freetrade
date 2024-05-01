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
    println!("input: {}, account: {}", input_file, account_name);
    let mut reader = csv::Reader::from_path(input_file)?;
    for result in reader.deserialize() {
        let record: Record = result?;
        println!("{:?}", record);
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
