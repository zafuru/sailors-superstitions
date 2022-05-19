use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    input_file: String,
}

fn main() {
    let args = Args::parse();
    println!("{}", args.input_file)
}
