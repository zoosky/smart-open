#![crate_name = "smart_open"]
extern crate flate2;
#[macro_use]
extern crate log;
extern crate reqwest;

use flate2::read::GzDecoder;
use reqwest::Error as req_error;
use s3::bucket::Bucket;
use s3::credentials::Credentials;
use s3::region::Region;
use std::fs::File;
use std::io::{BufReader, Read, Result};
use std::path::Path;
use std::result::Result as StdRes;
use std::str;
use std::string::String;

#[derive(Debug)]
struct S3Filepath {
    bucket: String,
    key: String,
}

fn parse_s3_filepaths(filepath: &str) -> S3Filepath {
    let split: Vec<&str> = filepath.split("/").collect();
    S3Filepath {
        bucket: split[2].to_string(),
        key: split[3..].join("/"),
    }
}

fn open_s3(filepath: &str) -> Result<String> {
    let s3_filepath: S3Filepath = parse_s3_filepaths(&filepath);

    // TODO should probably implement a strategy.
    // So that this vector is realigned to the median
    // of the last 10 runs.
    let mut regions = vec![
        Region::DoAms3,
        Region::DoNyc3,
        Region::UsEast1,
        Region::UsEast2,
        Region::UsWest1,
        Region::UsWest2,
        Region::CaCentral1,
        Region::ApSouth1,
        Region::ApNortheast1,
        Region::ApNortheast2,
        Region::ApNortheast3,
        Region::ApSoutheast1,
        Region::ApSoutheast2,
        Region::CnNorth1,
        Region::CnNorthwest1,
        Region::EuCentral1,
        Region::EuWest1,
        Region::EuWest2,
        Region::EuWest3,
        Region::SaEast1,
        Region::DoNyc3,
        Region::DoAms3,
        Region::DoSgp1,
    ];
    let result = loop {
        match regions.pop() {
            Some(r) => {
                // Create Bucket in REGION for BUCKET
                let credentials = Credentials::default();
                let bucket = Bucket::new(&s3_filepath.bucket, r.clone(), credentials);
                if let Ok((data, code)) = bucket.get(&s3_filepath.key) {
                    if code == 200 {
                        break data;
                    }
                } else {
                    debug!("failed with {}", r);
                }
            }
            _ => panic!("All the regions have been exhausted."),
        }
    };
    let string = str::from_utf8(&result).unwrap();
    Ok(string.to_string())
}

fn parse_gzip_buffer_to_string(buf: &mut Vec<u8>) -> Result<String> {
    let mut gz = GzDecoder::new(&buf[..]);
    let mut s = String::new();
    match gz.read_to_string(&mut s) {
        Ok(v) => v,
        Err(_) => panic!("Not able to gunzip the read resource. Please check."),
    };
    Ok(s)
}

fn open_http(filepath: &str, content_type: &str) -> StdRes<String, req_error> {
    let resp = reqwest::get(filepath)?;
    let mut verified_response = if resp.status().is_success() {
        resp
    } else {
        panic!("The file {} not parsed correctly. Please check", filepath);
    };
    let mut buf: Vec<u8> = vec![];
    verified_response.copy_to(&mut buf)?;
    let string = match content_type.as_ref() {
        "gz" => parse_gzip_buffer_to_string(&mut buf).unwrap(),
        // "text" => {
        //     let s = match str::from_utf8(&mut buf) {
        //         Ok(v) => v,
        //         Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        //     };
        //     s
        // }
        e => panic!("Case not handled: {}", e),
    };
    Ok(string.to_string())
}

fn pass_to_appropriate_function_for_content(
    filepath: &str,
    path: &Path,
    content_type: &str,
) -> Result<String> {
    let mut contents = String::new();
    if filepath.starts_with("s3://") {
        contents = open_s3(&filepath).unwrap();
    } else if filepath.starts_with("http") {
        contents = open_http(&filepath, content_type).unwrap();
    } else {
        let file_handler = File::open(&path)?;
        let mut buf_reader = BufReader::new(file_handler);
        if content_type == "gz" {
            let mut reader = GzDecoder::new(buf_reader);
            reader.read_to_string(&mut contents).unwrap();
        } else if content_type == "text" {
            buf_reader.read_to_string(&mut contents)?;
        } else {
            panic!("Content type {} is not allowed.", content_type);
        }
    }
    Ok(contents.trim().to_string())
}

/// Returns the contents of the file that has been passed as filepath
///
/// # Examples
///
/// ```
/// let filepath = "tests/foo.txt";
/// assert_eq!(smart_open::smart_open(&filepath).unwrap(), "Hello, world!");
/// ```
pub fn smart_open(filepath: &str) -> std::io::Result<String> {
    // TODO: make the function more modular.
    let path = Path::new(&filepath);
    let content_type = match path.extension() {
        // OK so I have decided to include file extensions without paths.
        None => "text",
        Some(os_str) => match os_str.to_str() {
            None => panic!("None has seeped in somehow. Please contact developers."),
            Some("gz") => "gz",
            _ => "text",
        },
    };
    pass_to_appropriate_function_for_content(filepath, path, content_type)
}
