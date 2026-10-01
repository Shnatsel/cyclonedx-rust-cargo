use cyclonedx_bom::{models::bom::{Bom, SpecVersion}, validation::Validate};
use std::{fs, path::Path, str::FromStr};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let spec_root = args.get(1).map(String::as_str).unwrap_or("/home/shnatsel/Code/cyclonedx-specification");
    let root = Path::new(spec_root).join("tools/src/test/resources");
    let out = Path::new(args.get(2).map(String::as_str).unwrap_or("/tmp/cyclonedx-pr872-review/probe-output"));
    let versions: Vec<&str> = if args.len() > 3 {
        args[3..].iter().map(String::as_str).collect()
    } else {
        vec!["1.3", "1.4", "1.5", "1.6", "1.7"]
    };
    let mut results = Vec::new();
    fs::create_dir_all(out).unwrap();
    for version in versions {
        let spec = SpecVersion::from_str(version).unwrap();
        for entry in fs::read_dir(root.join(version)).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_str().unwrap();
            if !name.starts_with("valid") { continue; }
            let ext = path.extension().unwrap().to_str().unwrap();
            let bytes = fs::read(&path).unwrap();
            let parsed = match ext {
                "json" => Bom::parse_from_json_with_version(&bytes[..], spec).map_err(|e| e.to_string()),
                "xml" => Bom::parse_from_xml_with_version(&bytes[..], spec).map_err(|e| e.to_string()),
                _ => continue,
            };
            let mut result = serde_json::json!({"version":version,"input":path,"name":name});
            match parsed {
                Err(e) => {
                    println!("PARSE {version}/{name}: {e}");
                    result["parse_error"] = serde_json::json!(e);
                },
                Ok(bom) => {
                    let validation = bom.validate_version(spec);
                    if !validation.passed() { println!("VALIDATE {version}/{name}: {validation:?}"); }
                    let mut json = Vec::new();
                    match bom.clone().output_as_json(&mut json, spec) {
                        Ok(()) => {
                            let filename = format!("{version}-{name}.json");
                            fs::write(out.join(&filename), json).unwrap();
                            result["json"] = serde_json::json!(filename);
                        },
                        Err(e) => println!("WRITE_JSON {version}/{name}: {e}"),
                    }
                    let mut xml = Vec::new();
                    match bom.output_as_xml(&mut xml, spec) {
                        Ok(()) => {
                            let filename = format!("{version}-{name}.xml");
                            fs::write(out.join(&filename), xml).unwrap();
                            result["xml"] = serde_json::json!(filename);
                        },
                        Err(e) => {
                            println!("WRITE_XML {version}/{name}: {e}");
                            result["xml_error"] = serde_json::json!(e.to_string());
                        },
                    }
                }
            }
            results.push(result);
        }
    }
    results.sort_by(|a,b| a["input"].as_str().cmp(&b["input"].as_str()));
    fs::write(out.join("manifest.json"), serde_json::to_vec_pretty(&results).unwrap()).unwrap();
}
