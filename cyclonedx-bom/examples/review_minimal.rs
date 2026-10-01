use cyclonedx_bom::{models::bom::{Bom, SpecVersion}, validation::Validate};
use serde_json::json;
use std::{fs, path::Path};

fn main() {
    let out = Path::new("/tmp/cyclonedx-pr872-review/minimal-output");
    fs::create_dir_all(out).unwrap();
    for (name, spec, mut input) in [
        ("component-order", SpecVersion::V1_3, json!({"components":[{"type":"library","name":"parent","version":"1","properties":[{"name":"key","value":"value"}],"components":[{"type":"library","name":"child","version":"1"}]}]})),
        ("service-order", SpecVersion::V1_3, json!({"services":[{"name":"parent","properties":[{"name":"key","value":"value"}],"services":[{"name":"child"}]}]})),
        ("bom-order", SpecVersion::V1_5, json!({"properties":[{"name":"key","value":"value"}],"vulnerabilities":[{"id":"CVE-2026-1234"}]})),
        ("manufacturer-order", SpecVersion::V1_6, json!({"metadata":{"manufacturer":{"name":"maker"},"authors":[{"name":"author"}]}})),
        ("external", SpecVersion::V1_7, json!({"components":[{"type":"library","name":"lib","isExternal":true}]})),
        ("crypto", SpecVersion::V1_6, json!({"components":[{"type":"cryptographic-asset","name":"AES","cryptoProperties":{"assetType":"algorithm","algorithmProperties":{"primitive":"block-cipher","classicalSecurityLevel":128,"cryptoFunctions":["encrypt"]}}}]})),
        ("identity", SpecVersion::V1_6, json!({"components":[{"type":"library","name":"lib","evidence":{"identity":[{"field":"name","confidence":1.0}]}}]})),
        ("provides", SpecVersion::V1_6, json!({"dependencies":[{"ref":"library","provides":["algorithm"]}]})),
    ] {
        input["bomFormat"] = json!("CycloneDX");
        input["specVersion"] = json!(spec.to_string());
        input["version"] = json!(1);
        test_utils::validate_json_with_schema(&input,spec).unwrap();
        let bytes=serde_json::to_vec(&input).unwrap();
        fs::write(out.join(format!("{name}.input.json")),&bytes).unwrap();
        let bom=match Bom::parse_from_json_with_version(&bytes[..],spec) {
            Ok(b)=>b, Err(e)=>{println!("{name}: PARSE ERROR {e}");continue;}
        };
        println!("{name}: validation {:?}",bom.validate_version(spec));
        let mut bytes=Vec::new();
        bom.clone().output_as_json(&mut bytes,spec).unwrap();
        fs::write(out.join(format!("{name}.output.json")),&bytes).unwrap();
        let mut bytes=Vec::new();
        bom.output_as_xml(&mut bytes,spec).unwrap();
        fs::write(out.join(format!("{name}.output.xml")),&bytes).unwrap();
        match Bom::parse_from_xml_with_version(&bytes[..],spec) {
            Ok(bom)=> {
                let mut bytes=Vec::new();
                bom.output_as_json(&mut bytes,spec).unwrap();
                fs::write(out.join(format!("{name}.roundtrip.json")),&bytes).unwrap();
            },
            Err(e)=>println!("{name}: ROUNDTRIP PARSE ERROR {e}"),
        }
    }
}
