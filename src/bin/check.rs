use anyhow::{Context, Result};
use layered_nft_gen::config::{Config, ForbiddenPair};
use layered_nft_gen::metadata::NftMetadata;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    let metadata_dir = Path::new("output/metadata");

    let cfg = Config::load("config.yaml").ok();

    let forbidden_pairs: &[ForbiddenPair] = cfg
        .as_ref()
        .and_then(|c| c.constraints.as_ref())
        .and_then(|c| c.forbidden_pairs.as_ref())
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    let mut total = 0usize;
    let mut stats: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut violation_count = 0usize;
    let mut violation_examples: Vec<(String, String)> = Vec::new();
    let max_examples = 20usize;

    for entry in fs::read_dir(metadata_dir)
        .with_context(|| format!("metadata ディレクトリが読めません: {:?}", metadata_dir))?
    {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let text = fs::read_to_string(&path)
            .with_context(|| format!("JSON 読み込み失敗: {:?}", path))?;
        let meta: NftMetadata = serde_json::from_str(&text)
            .with_context(|| format!("JSON パース失敗: {:?}", path))?;

        total += 1;

        for attr in &meta.attributes {
            let value_map = stats
                .entry(attr.trait_type.clone())
                .or_insert_with(HashMap::new);
            *value_map.entry(attr.value.clone()).or_insert(0) += 1;
        }

        if !forbidden_pairs.is_empty() {
            let present: HashSet<(&str, &str)> = meta
                .attributes
                .iter()
                .map(|a| (a.trait_type.as_str(), a.value.as_str()))
                .collect();

            let mut violated_this_token = false;

            for p in forbidden_pairs {
                let a = (p.a.trait_type.as_str(), p.a.value.as_str());
                let b = (p.b.trait_type.as_str(), p.b.value.as_str());

                if present.contains(&a) && present.contains(&b) {
                    violated_this_token = true;

                    if violation_examples.len() < max_examples {
                        let file = path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("<unknown>")
                            .to_string();

                        let msg = format!(
                            "forbidden pair matched: ({}/{}) + ({}/{})",
                            p.a.trait_type, p.a.value, p.b.trait_type, p.b.value
                        );
                        violation_examples.push((file, msg));
                    }

                    break;
                }
            }

            if violated_this_token {
                violation_count += 1;
            }
        }
    }

    println!("==============================");
    println!(" NFT Rarity Check");
    println!(" Total tokens: {}", total);
    println!("==============================\n");

    for (trait_type, values) in stats {
        println!("▶ Trait: {}", trait_type);

        let mut sorted: Vec<_> = values.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        for (value, count) in sorted {
            let ratio = count as f64 / total as f64 * 100.0;
            println!("  {:30} {:5} ({:.2}%)", value, count, ratio);
        }
        println!();
    }

    if forbidden_pairs.is_empty() {
        println!("(constraints.forbidden_pairs が未設定のため、禁則チェックはスキップしました)");
    } else {
        println!("==============================");
        println!(" Forbidden-pairs Check");
        println!(" Violations(tokens): {}", violation_count);
        println!("==============================");

        if violation_count == 0 {
            println!("✅ 禁則違反は見つかりませんでした");
        } else {
            println!("❌ 禁則違反が見つかりました（最大 {} 件表示）:", max_examples);
            for (file, msg) in &violation_examples {
                println!("  - {} : {}", file, msg);
            }
        }
    }

    if violation_count > 0 {
        std::process::exit(1);
    }

    Ok(())
}
