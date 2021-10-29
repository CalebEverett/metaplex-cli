use arweave_rs::{
    transaction::{Base64, Tag},
    Arweave, Error, Methods as ArewaveMethods, StatusCode,
};
use glob::glob;
use std::{iter, path::PathBuf, str::FromStr};

async fn get_arweave() -> Result<Arweave, Error> {
    let keypair_path =
        "tests/fixtures/arweave-keyfile-MlV6DeOtRmakDOf6vgOBlif795tcWimgyPsYYNQ8q1Y.json";
    let base_url = "http://localhost:1984/";
    let arweave = Arweave::from_keypair_path(PathBuf::from(keypair_path), Some(base_url)).await?;
    Ok(arweave)
}

#[tokio::test]
async fn test_post_transaction() -> Result<(), Error> {
    let arweave = get_arweave().await?;
    // Don't run if test server is not running.
    if let Err(_) = reqwest::get(arweave.base_url.join("info")?).await {
        println!("Test server not running.");
        return Ok(());
    }

    let file_path = PathBuf::from("tests/fixtures/0.png");
    let last_tx = Base64::from_str("LCwsLCwsLA")?;
    let transaction = arweave
        .create_transaction_from_file_path(file_path, None, Some(last_tx), Some(0))
        .await?;

    let signed_transaction = arweave.sign_transaction(transaction)?;
    println!("signed_transaction: {:?}", &signed_transaction);
    arweave.post_transaction(&signed_transaction, None).await?;

    let url = arweave.base_url.join("mine")?;
    let resp = reqwest::get(url).await?.text().await?;
    println!("mine: {}", resp);

    let status = arweave.get_raw_status(&signed_transaction.id).await?;
    println!("{:?}", status);
    Ok(())
}

#[tokio::test]
async fn test_upload_file_from_path() -> Result<(), Error> {
    let arweave = get_arweave().await?;
    // Don't run if test server is not running.
    if let Err(_) = reqwest::get(arweave.base_url.join("info")?).await {
        println!("Test server not running.");
        return Ok(());
    }

    let file_path = PathBuf::from("tests/fixtures/0.png");
    let last_tx = Base64::from_str("LCwsLCwsLA")?;
    let log_dir = PathBuf::from("../target/tmp");

    let status = arweave
        .upload_file_from_path(
            file_path.clone(),
            Some(log_dir.clone()),
            None,
            Some(last_tx),
            Some(0),
        )
        .await?;

    let read_status = arweave.read_status(file_path, log_dir).await?;
    println!("{:?}", &read_status);
    assert_eq!(status, read_status);

    Ok(())
}

#[tokio::test]
async fn test_update_status() -> Result<(), Error> {
    let arweave = get_arweave().await?;
    // Don't run if test server is not running.
    if let Err(_) = reqwest::get(arweave.base_url.join("info")?).await {
        println!("Test server not running.");
        return Ok(());
    }

    let file_path = PathBuf::from("tests/fixtures/0.png");
    let last_tx = Base64::from_str("LCwsLCwsLA")?;
    let log_dir = PathBuf::from("../target/tmp");

    let _ = arweave
        .upload_file_from_path(
            file_path.clone(),
            Some(log_dir.clone()),
            None,
            Some(last_tx),
            Some(0),
        )
        .await?;

    let read_status = arweave
        .read_status(file_path.clone(), log_dir.clone())
        .await?;
    assert_eq!(read_status.status, StatusCode::Submitted);

    let url = arweave.base_url.join("mine")?;
    let resp = reqwest::get(url).await?.text().await?;
    println!("mine resp: {}", resp);

    let updated_status = arweave.update_status(file_path, log_dir).await?;
    println!("{:?}", &updated_status);
    assert_eq!(updated_status.status, StatusCode::Confirmed);
    assert!(updated_status.last_modified > read_status.last_modified);
    Ok(())
}

#[tokio::test]
async fn test_upload_files_from_paths_without_tags() -> Result<(), Error> {
    let arweave = get_arweave().await?;
    // Don't run if test server is not running.
    if let Err(_) = reqwest::get(arweave.base_url.join("info")?).await {
        println!("Test server not running.");
        return Ok(());
    }

    let paths_iter = glob("tests/fixtures/*.png")?.filter_map(Result::ok);
    let last_tx = Some(Base64::from_str("LCwsLCwsLA")?);
    let log_dir = Some(PathBuf::from("../target/tmp"));
    let reward = Some(0);
    let mut tags_iter = Some(iter::repeat(Some(Vec::<Tag>::new())));
    tags_iter = None;

    let statuses = arweave
        .upload_files_from_paths(paths_iter, log_dir.clone(), tags_iter, last_tx, reward)
        .await?;

    let paths_iter = glob("tests/fixtures/*.png")?.filter_map(Result::ok);
    let read_statuses = arweave.read_statuses(paths_iter, log_dir.unwrap()).await?;
    assert_eq!(statuses, read_statuses);
    Ok(())
}

#[tokio::test]
async fn test_update_statuses() -> Result<(), Error> {
    let arweave = get_arweave().await?;
    // Don't run if test server is not running.
    if let Err(_) = reqwest::get(arweave.base_url.join("info")?).await {
        println!("Test server not running.");
        return Ok(());
    }

    let paths_iter = glob("tests/fixtures/*.png")?.filter_map(Result::ok);
    let last_tx = Some(Base64::from_str("LCwsLCwsLA")?);
    let log_dir = Some(PathBuf::from("../target/tmp"));
    let reward = Some(0);
    let mut tags_iter = Some(iter::repeat(Some(Vec::<Tag>::new())));
    tags_iter = None;

    let statuses = arweave
        .upload_files_from_paths(paths_iter, log_dir.clone(), tags_iter, last_tx, reward)
        .await?;

    println!("{:?}", statuses);
    let url = arweave.base_url.join("mine")?;
    let resp = reqwest::get(url).await?.text().await?;
    println!("mine resp: {}", resp);

    let paths_iter = glob("tests/fixtures/*.png")?.filter_map(Result::ok);
    let read_statuses = arweave.read_statuses(paths_iter, log_dir.unwrap()).await?;

    println!("{:?}", read_statuses);

    let all_confirmed = read_statuses
        .iter()
        .all(|s| s.status == StatusCode::Confirmed);
    assert!(all_confirmed);
    Ok(())
}
