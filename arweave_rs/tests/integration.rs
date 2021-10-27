use arweave_rs::{transaction::Base64, Arweave, Error, Methods as ArewaveMethods};
use std::str::FromStr;

#[tokio::test]
async fn test_post_transaction() -> Result<(), Error> {
    let keypair_path =
        "tests/fixtures/arweave-keyfile-MlV6DeOtRmakDOf6vgOBlif795tcWimgyPsYYNQ8q1Y.json";
    let base_url = "http://localhost:1984/";
    let arweave = Arweave::from_keypair_path(keypair_path, Some(base_url)).await?;

    // Don't run if test server is not running.
    if let Err(_) = reqwest::get(arweave.base_url.join("info")?).await {
        println!("Test server not runing.");
        return Ok(());
    }

    let file_path = "tests/fixtures/0.png";
    let last_tx = Base64::from_str("LCwsLCwsLA")?;
    let transaction = arweave
        .create_transaction_from_file_path(file_path, None, Some(last_tx), None)
        .await?;

    let signed_transaction = arweave.sign_transaction(transaction)?;
    arweave.post_transaction(&signed_transaction).await?;

    let url = arweave.base_url.join("mine")?;
    let resp = reqwest::get(url).await?.text().await?;
    println!("mine: {}", resp);

    let _ = arweave.check_status(&signed_transaction.id).await?;
    Ok(())
}