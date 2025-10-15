// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

use assert_cmd::Command;
use base64::Engine as _;
use std::{collections::HashMap, env};

#[test]
fn test_version() -> Result<(), Box<dyn std::error::Error>> {
    println!("STUDIO_SERVER: {:?}", env::var("STUDIO_SERVER"));
    println!("STUDIO_TOKEN: {:?}", env::var("STUDIO_TOKEN"));
    println!("STUDIO_USERNAME: {:?}", env::var("STUDIO_USERNAME"));
    println!("STUDIO_PASSWORD: {:?}", env::var("STUDIO_PASSWORD"));

    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("version");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains(env!("CARGO_PKG_VERSION")));
    Ok(())
}

#[test]
fn test_token() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("token");

    let token = cmd.ok()?.stdout;
    assert!(!token.is_empty());

    println!("Token: {}", String::from_utf8_lossy(&token));

    let token = String::from_utf8(token)?;
    let token_parts: Vec<&str> = token.split('.').collect();
    assert_eq!(token_parts.len(), 3);

    let decoded = base64::engine::general_purpose::STANDARD_NO_PAD
        .decode(token_parts[1])
        .unwrap();
    let payload: HashMap<String, serde_json::Value> = serde_json::from_slice(&decoded)?;
    let username = payload.get("username");
    assert!(username.is_some());
    let username = username.unwrap().as_str().unwrap();
    assert!(!username.is_empty());

    match env::var("STUDIO_USERNAME") {
        Ok(studio_username) => assert_eq!(studio_username, username),
        Err(_) => {}
    }

    Ok(())
}

#[test]
fn test_organization() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("edgefirst-client")?;
    cmd.arg("organization");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Organization:"));
    Ok(())
}
