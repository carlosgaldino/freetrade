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
