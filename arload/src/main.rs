use clap::{
    self, crate_description, crate_name, crate_version, value_t, App, AppSettings, Arg, SubCommand,
    Values,
};

use arload::{
    error::ArweaveError,
    transaction::{Base64, FromStrs, Tag},
    Arweave, Methods as ArweaveMethods, WINSTONS_PER_AR,
};
use glob::glob;
use num_traits::cast::ToPrimitive;
use std::{fmt::Display, path::PathBuf, str::FromStr};

pub type CommandResult = Result<(), ArweaveError>;

fn get_app() -> App<'static, 'static> {
    let app_matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::with_name("keypair_path")
                .long("keypair-path")
                .value_name("ARWEAVE_KEYPAIR_PATH")
                .validator(is_parsable::<PathBuf>)
                .env("ARWEAVE_KEYPAIR_PATH")
                .required(true)
                .help(
                    "Specify path to keypair file for Arweave \
                        wallet to pay for and sign upload transaction. \
                        Defaults to value specified in \
                        ARWEAVE_KEYPAIR_PATH environment variable.",
                ),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .takes_value(false)
                .global(true)
                .help("Show additional information."),
        )
        .arg(
            Arg::with_name("output_format")
                .long("output")
                .value_name("FORMAT")
                .global(true)
                .takes_value(true)
                .possible_values(&["json", "json-compact"])
                .help("Return information in specified output format."),
        )
        .subcommand(
            SubCommand::with_name("price")
                .about("Returns the price of uploading data.")
                .arg(
                    Arg::with_name("bytes")
                        .value_name("BYTES")
                        .takes_value(true)
                        .validator(is_parsable::<u32>)
                        .help("Specify the number of bytes to to be uploaded."),
                ),
        )
        .subcommand(
            SubCommand::with_name("get-transaction")
                .about("Fetches file data from.")
                .arg(
                    Arg::with_name("id")
                        .value_name("ID")
                        .takes_value(true)
                        .validator(is_parsable::<Base64>)
                        .help("Id of data to return from storage."),
                ),
        )
        .subcommand(
            SubCommand::with_name("wallet-balance")
                .about("Returns the balance of a wallet.")
                .arg(
                    Arg::with_name("wallet_address")
                        .value_name("WALLET_ADDRESS")
                        .takes_value(true)
                        .validator(is_parsable::<Base64>)
                        .help(
                            "Specify wallet address for which to \
                    return balance. Defaults to address of keypair \
                    used by keypair-path argument.",
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("file-upload")
                .about("Uploads a single file.")
                .arg(
                    Arg::with_name("file_path")
                        .value_name("FILE_PATH")
                        .takes_value(true)
                        .required(true)
                        .validator(is_parsable::<PathBuf>)
                        .help("Path of the file to be uploaded."),
                )
                .arg(
                    Arg::with_name("log_dir")
                        .long("log-dir")
                        .value_name("LOG_DIR")
                        .takes_value(true)
                        .validator(is_parsable::<PathBuf>)
                        .help(
                            "Directory to write status updates to. If not \
                        provided, status updates will not be written.
                        ",
                        ),
                )
                .arg(
                    Arg::with_name("tags")
                        .long("tags")
                        .value_name("TAGS")
                        .multiple(true)
                        .takes_value(true)
                        .validator(is_valid_tag)
                        .help(
                            "Specify additional tags for the file as \
                            <NAME>:<VALUE>, separated by spaces. Content-Type tag \
                            will be inferred automatically so not necessary so \
                            include here.",
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("glob-upload")
                .about("Uploads files that match provided glob.")
                .arg(
                    Arg::with_name("glob")
                        .value_name("GLOB")
                        .takes_value(true)
                        .required(true)
                        .help(
                            "Glob pattern of files to be uploaded. \
                        Run glob-estimate to see how many files will be \
                        uploaded, total size and cost.",
                        ),
                )
                .arg(
                    Arg::with_name("log_dir")
                        .long("log-dir")
                        .value_name("LOG_DIR")
                        .takes_value(true)
                        .validator(is_parsable::<PathBuf>)
                        .help(
                            "Directory that status updates will be to. If not \
                        provided, status updates will not be written.
                        ",
                        ),
                )
                .arg(
                    Arg::with_name("tags")
                        .long("tags")
                        .value_name("TAGS")
                        .multiple(true)
                        .takes_value(true)
                        .validator(is_valid_tag)
                        .help(
                            "Specify additional tags for the files as \
                            <NAME>:<VALUE>, separated by spaces. Content-Type tag \
                            will be inferred automatically so not necessary so \
                            include here. Additional tags will be applied
                            to all of the uploaded files.",
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("status-raw")
                .about("Get raw transaction status from network.")
                .arg(
                    Arg::with_name("id")
                        .value_name("ID")
                        .takes_value(true)
                        .required(true)
                        .help("Id of transaction to check status on."),
                ),
        )
        .subcommand(
            SubCommand::with_name("status-update")
                .about(
                    "Query the network to fetch updated transaction status and \
                update stored transaction status on disk.",
                )
                .arg(
                    Arg::with_name("file_path")
                        .value_name("FILE_PATH")
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .validator(is_parsable::<PathBuf>)
                        .help("Path of previously uploaded file."),
                )
                .arg(
                    Arg::with_name("log_dir")
                        .long("log-dir")
                        .value_name("LOG_DIR")
                        .takes_value(true)
                        .required(true)
                        .index(2)
                        .validator(is_parsable::<PathBuf>)
                        .help("Directory that status update was written to."),
                ),
        );
    app_matches
}

fn is_parsable_generic<U, T>(string: T) -> Result<(), String>
where
    T: AsRef<str> + Display,
    U: FromStr,
    U::Err: Display,
{
    string
        .as_ref()
        .parse::<U>()
        .map(|_| ())
        .map_err(|err| format!("error parsing '{}': {}", string, err))
}

// Return an error if string cannot be parsed as type T.
// Takes a String to avoid second type parameter when used as a clap validator
pub fn is_parsable<T>(string: String) -> Result<(), String>
where
    T: FromStr,
    T::Err: Display,
{
    is_parsable_generic::<T, String>(string)
}

fn is_valid_tag<T>(tag: T) -> Result<(), String>
where
    T: AsRef<str> + Display,
{
    let split: Vec<_> = tag.as_ref().split(":").collect();
    match Tag::from_utf8_strs(split[0], split[1]) {
        Ok(_) => Ok(()),
        Err(_) => Err(format!("Not a valid tag.")),
    }
}

fn get_tags_vec(tag_values: Option<Values>) -> Option<Vec<Tag>> {
    if let Some(tag_strings) = tag_values {
        let tags = tag_strings
            .into_iter()
            .map(|t| {
                let split: Vec<&str> = t.split(":").collect();
                Tag::from_utf8_strs(split[0], split[1])
            })
            .flat_map(Result::ok)
            .collect();
        Some(tags)
    } else {
        None
    }
}

async fn command_price(arweave: &Arweave, bytes: &usize) -> CommandResult {
    let (winstons_per_bytes, usd_per_ar) = arweave.get_price(bytes).await?;
    let usd_per_kb = (&winstons_per_bytes * &usd_per_ar).to_f32().unwrap() / 1e14_f32;
    println!(
        "The price to upload {} bytes to {} is {} {} (${}).",
        bytes, arweave.name, winstons_per_bytes, arweave.units, usd_per_kb
    );
    Ok(())
}

async fn command_get_transaction(arweave: &Arweave, id: &str) -> CommandResult {
    let id = Base64::from_str(id)?;
    let transaction = arweave.get_transaction(&id).await?;
    println!("Fetched transaction {}", transaction.id);
    Ok(())
}

async fn command_get_raw_status(arweave: &Arweave, id: &str) -> CommandResult {
    let id = Base64::from_str(id)?;
    let resp = arweave.get_raw_status(&id).await?;
    println!("{}", resp.text().await?);
    Ok(())
}

async fn command_update_status(arweave: &Arweave, file_path: &str, log_dir: &str) -> CommandResult {
    let status = arweave
        .update_status(PathBuf::from(file_path), PathBuf::from(log_dir))
        .await?;
    println!("{}", serde_json::to_string_pretty(&status)?);
    Ok(())
}

async fn command_wallet_balance(
    arweave: &Arweave,
    wallet_address: Option<String>,
) -> CommandResult {
    let mb = u32::pow(1024, 2) as usize;
    let result = tokio::join!(
        arweave.get_wallet_balance(wallet_address),
        arweave.get_price(&mb)
    );
    let balance = result.0?;
    let (winstons_per_kb, usd_per_ar) = result.1?;

    let balance_usd = &balance / &WINSTONS_PER_AR * &usd_per_ar;

    let usd_per_kb = (&winstons_per_kb * &usd_per_ar).to_f32().unwrap() / 1e14_f32;

    println!(
            "Wallet balance is {} {units} (${balance_usd}). At the current price of {price} {units} (${usd_price:.4}) per MB, you can upload {max} MB of data.",
            &balance,
            units = arweave.units,
            max = &balance / &winstons_per_kb,
            price = &winstons_per_kb,
            balance_usd = balance_usd.to_f32().unwrap() / 100_f32,
            usd_price = usd_per_kb
    );
    Ok(())
}

async fn command_file_upload(
    arweave: &Arweave,
    file_path: &str,
    log_dir: Option<&str>,
    tags: Option<Vec<Tag>>,
) -> CommandResult {
    let status = arweave
        .upload_file_from_path(
            PathBuf::from(file_path),
            log_dir.map(|v| PathBuf::from(v)),
            tags,
            None,
            None,
        )
        .await?;

    println!("{}", serde_json::to_string_pretty(&status)?);
    Ok(())
}

async fn command_glob_upload(
    arweave: &Arweave,
    glob_str: &str,
    log_dir: Option<&str>,
    tags: Option<Vec<Tag>>,
) -> CommandResult {
    let paths_iter = glob(glob_str)?.filter_map(Result::ok);
    let log_dir = log_dir.map(|s| PathBuf::from(s));

    // let _ = try_join_all(paths_iter.for_each(|p| {
    //     let status = arweave.upload_file_from_path(p, log_dir.clone(), tags.clone(), None, None);

    // }))
    // .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> CommandResult {
    let app_matches = get_app().get_matches();
    let keypair_path = app_matches.value_of("keypair_path").unwrap();

    let arweave = Arweave::from_keypair_path(PathBuf::from(keypair_path), None)
        .await
        .unwrap();

    let (sub_command, arg_matches) = app_matches.subcommand();

    match (sub_command, arg_matches) {
        ("price", Some(sub_arg_matches)) => {
            let bytes = value_t!(sub_arg_matches, "bytes", usize).unwrap();
            command_price(&arweave, &bytes).await
        }
        ("get-transaction", Some(sub_arg_matches)) => {
            let id = sub_arg_matches.value_of("id").unwrap();
            command_get_transaction(&arweave, id).await
        }
        ("wallet-balance", Some(sub_arg_matches)) => {
            let wallet_address = sub_arg_matches
                .value_of("wallet_address")
                .map(|v| v.to_string());
            command_wallet_balance(&arweave, wallet_address).await
        }
        ("file-upload", Some(sub_arg_matches)) => {
            let file_path = sub_arg_matches.value_of("file_path").unwrap();
            let log_dir = sub_arg_matches.value_of("log_dir");
            let tags = get_tags_vec(sub_arg_matches.values_of("tags"));
            command_file_upload(&arweave, file_path, log_dir, tags).await
        }
        ("glob-upload", Some(sub_arg_matches)) => {
            let glob_str = sub_arg_matches.value_of("glob").unwrap();
            let log_dir = sub_arg_matches.value_of("log_dir");
            let tags = get_tags_vec(sub_arg_matches.values_of("tags"));
            command_glob_upload(&arweave, glob_str, log_dir, tags).await
        }
        ("status-raw", Some(sub_arg_matches)) => {
            let id = sub_arg_matches.value_of("id").unwrap();
            command_get_raw_status(&arweave, id).await
        }
        ("status-update", Some(sub_arg_matches)) => {
            let file_path = sub_arg_matches.value_of("file_path").unwrap();
            let log_dir = sub_arg_matches.value_of("log_dir").unwrap();
            command_update_status(&arweave, file_path, log_dir).await
        }
        _ => unreachable!(),
    }
}
