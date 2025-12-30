use layered_nft_gen::config::{Config, LayerConfig, MetadataConfig};
use layered_nft_gen::metadata::{Attribute, NftMetadata};

use anyhow::{bail, Context, Result};
use image::{ImageBuffer, RgbaImage};
use rand::distributions::WeightedIndex;
use rand::prelude::*;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;
use oxipng::{InFile, OutFile, Options};

/// 1トークン生成時に選ばれたレイヤー1枚分
struct LayerChoice {
    path: PathBuf,
    trait_type: String,
    value: String,
}

/// 各レイヤー種別の候補一覧
struct LayerCandidate<'a> {
    layer: &'a LayerConfig,
    files: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let cfg = Config::load("config.yaml")
        .context("config.yaml の読み込みに失敗しました")?;

    fs::create_dir_all(&cfg.output.image_dir)
        .with_context(|| format!("画像出力ディレクトリの作成に失敗しました: {}", cfg.output.image_dir))?;
    fs::create_dir_all(&cfg.output.metadata_dir)
        .with_context(|| format!("メタデータ出力ディレクトリの作成に失敗しました: {}", cfg.output.metadata_dir))?;

    let mut layer_candidates: Vec<LayerCandidate> = Vec::new();

    for layer in &cfg.layers {
        let dir_path = Path::new(&layer.directory);
        let files = collect_png_files(dir_path)
            .with_context(|| format!("レイヤーディレクトリの走査に失敗しました: {:?}", dir_path))?;

        if files.is_empty() {
            bail!(
                "レイヤー {:?} ({:?}) に PNG ファイルがありません",
                layer.name,
                dir_path
            );
        }

        layer_candidates.push(LayerCandidate { layer, files });
    }

    let total_combinations: u128 = layer_candidates
        .iter()
        .map(|c| c.files.len() as u128)
        .product();

    if cfg.count as u128 > total_combinations {
        bail!(
            "要求された生成数 {} は理論上の最大組み合わせ数 {} を超えています。\
             レイヤーのバリエーションを増やすか、count を減らしてください。",
            cfg.count,
            total_combinations
        );
    }

    println!(
        "Generating {} NFTs in parallel (max unique patterns: {})...",
        cfg.count, total_combinations
    );

    let used_patterns: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    (1..=cfg.count)
        .into_par_iter()
        .for_each(|token_id| {
            if let Err(err) =
                generate_one(token_id, &cfg, &layer_candidates, &used_patterns)
            {
                eprintln!("❌ Error in token #{}: {:?}", token_id, err);
            }
        });

    println!("✅ All tokens generated without duplication!");

    Ok(())
}

/// 1トークン分を生成する処理（並列で呼ばれる）
fn generate_one(
    token_id: u32,
    cfg: &Config,
    layer_candidates: &[LayerCandidate],
    used_patterns: &Arc<Mutex<HashSet<String>>>,
) -> Result<()> {
    const MAX_RETRY: u32 = 1000;

    let mut rng = thread_rng();
    let mut chosen_layers: Vec<LayerChoice> = Vec::new();
    let pattern_key: String;

    'retry_loop: {
        for _attempt in 0..MAX_RETRY {
            chosen_layers.clear();

            for candidate in layer_candidates {
                let chosen_path =
                    choose_layer_file_with_rng(&candidate.files, &candidate.layer.rarity, &mut rng);
                let value =
                    file_stem(&chosen_path).unwrap_or_else(|| "Unknown".to_string());

                chosen_layers.push(LayerChoice {
                    path: chosen_path,
                    trait_type: candidate.layer.name.clone(),
                    value,
                });
            }

            if violates_constraints(cfg, &chosen_layers) {
                continue;
            }

            let key = build_pattern_key(&chosen_layers);

            {
                let mut set = used_patterns
                    .lock()
                    .expect("used_patterns のロックに失敗しました");
                if !set.contains(&key) {
                    set.insert(key.clone());
                    pattern_key = key;
                    break 'retry_loop;
                }
            }
        }

        bail!(
            "トークン #{} で一意なパターンを見つけられませんでした（MAX_RETRY超過）。\
             count が組み合わせ数ギリギリか、rarity 設定が極端な可能性があります。",
            token_id
        );
    }

    let composed = compose_layers(&chosen_layers)
        .with_context(|| format!("トークン #{} の画像合成に失敗しました", token_id))?;

    let image_path = format!("{}/{}.png", cfg.output.image_dir, token_id);
    composed
        .save(&image_path)
        .with_context(|| format!("画像の保存に失敗しました: {}", image_path))?;

    if let Some(c) = &cfg.output.png_compression {
        if c.enabled {
            compress_png(&image_path, c.level)
                .with_context(|| format!("PNG 圧縮に失敗しました: {}", image_path))?;
        }
    }

    let metadata =
        build_metadata(token_id, &cfg.metadata, &chosen_layers);
    let metadata_path = format!("{}/{}.json", cfg.output.metadata_dir, token_id);
    let json = serde_json::to_string_pretty(&metadata)
        .context("メタデータのJSONシリアライズに失敗しました")?;
    fs::write(&metadata_path, json)
        .with_context(|| format!("メタデータの書き込みに失敗しました: {}", metadata_path))?;

    println!(
        "✅ token #{} -> {}, {} (pattern: {})",
        token_id, image_path, metadata_path, pattern_key
    );

    Ok(())
}


/// 禁則ルール判定
fn violates_constraints(cfg: &Config, layers: &[LayerChoice]) -> bool {
    let Some(c) = &cfg.constraints else { return false; };
    let Some(pairs) = &c.forbidden_pairs else { return false; };

    let present: HashSet<(String, String)> = layers
        .iter()
        .map(|l| (l.trait_type.clone(), l.value.clone()))
        .collect();

    for p in pairs {
        let a = (p.a.trait_type.clone(), p.a.value.clone());
        let b = (p.b.trait_type.clone(), p.b.value.clone());

        if present.contains(&a) && present.contains(&b) {
            return true;
        }
    }

    false
}

/// ディレクトリ以下の PNG ファイルを列挙
fn collect_png_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.into_path();
        if let Some(ext) = path.extension() {
            if ext.eq_ignore_ascii_case("png") {
                files.push(path);
            }
        }
    }
    Ok(files)
}

/// パスから拡張子抜きのファイル名を取得
fn file_stem(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

/// レイヤー組み合わせを一意に表すキーを作成
/// ここではフルパス文字列を "|" で連結している
fn build_pattern_key(layers: &[LayerChoice]) -> String {
    let mut parts = Vec::with_capacity(layers.len());
    for l in layers {
        let p = l.path.to_string_lossy();
        parts.push(p.to_string());
    }
    parts.join("|")
}

/// レア度テーブル付きの重み付きランダム選択
fn choose_layer_file_with_rng(
    files: &[PathBuf],
    rarity: &Option<HashMap<String, f32>>,
    rng: &mut ThreadRng,
) -> PathBuf {
    if let Some(rarity_map) = rarity {
        let weights: Vec<f32> = files
            .iter()
            .map(|path| {
                let file_name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                *rarity_map.get(file_name).unwrap_or(&1.0)
            })
            .collect();

        if let Ok(dist) = WeightedIndex::new(weights.iter().cloned()) {
            let idx = dist.sample(rng);
            return files[idx].clone();
        } else {
            eprintln!("⚠ レア度設定が不正です。均等ランダムにフォールバックします。");
        }
    }

    files
        .choose(rng)
        .expect("レイヤーファイルが空です")
        .clone()
}

/// PNG レイヤーを順に重ねて1枚にする
fn compose_layers(layers: &[LayerChoice]) -> Result<RgbaImage> {
    if layers.is_empty() {
        bail!("レイヤーが1枚も指定されていません");
    }

    let first = image::open(&layers[0].path)
        .with_context(|| format!("画像の読み込みに失敗しました: {:?}", layers[0].path))?
        .into_rgba8();
    let (width, height) = first.dimensions();

    let mut base: RgbaImage =
        ImageBuffer::from_fn(width, height, |x, y| *first.get_pixel(x, y));

    for layer in &layers[1..] {
        let img = image::open(&layer.path)
            .with_context(|| format!("画像の読み込みに失敗しました: {:?}", layer.path))?
            .into_rgba8();

        if img.width() != width || img.height() != height {
            bail!(
                "レイヤーのサイズが一致しません: {:?} ({}, {}) != base ({}, {})",
                layer.path,
                img.width(),
                img.height(),
                width,
                height
            );
        }

        overlay_rgba(&mut base, &img);
    }

    Ok(base)
}

/// base の上に overlay をαブレンドで重ねる
fn overlay_rgba(base: &mut RgbaImage, overlay: &RgbaImage) {
    for (x, y, pixel) in overlay.enumerate_pixels() {
        let [or, og, ob, oa] = pixel.0;
        let alpha = oa as f32 / 255.0;
        if alpha == 0.0 {
            continue;
        }

        let base_pixel = base.get_pixel_mut(x, y);
        let [br, bg, bb, ba] = base_pixel.0;

        let ba_f = ba as f32 / 255.0;
        let out_a = alpha + ba_f * (1.0 - alpha);

        let blend = |oc: u8, bc: u8| -> u8 {
            let oc_f = oc as f32 / 255.0;
            let bc_f = bc as f32 / 255.0;
            let out = if out_a == 0.0 {
                0.0
            } else {
                (oc_f * alpha + bc_f * ba_f * (1.0 - alpha)) / out_a
            };
            (out * 255.0).round().clamp(0.0, 255.0) as u8
        };

        let out_r = blend(or, br);
        let out_g = blend(og, bg);
        let out_b = blend(ob, bb);
        let out_a_u8 = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;

        *base_pixel = image::Rgba([out_r, out_g, out_b, out_a_u8]);
    }
}

/// NFT メタデータを構築
fn build_metadata(
    token_id: u32,
    metadata_config: &MetadataConfig,
    layers: &[LayerChoice],
) -> NftMetadata {
    let name = if metadata_config.name.is_empty() {
        format!("#{}", token_id)
    } else {
        format!("{} #{}", metadata_config.name, token_id)
    };
    let description = metadata_config.description.clone();
    let image = format!("{}/{}.png", metadata_config.base_image_url, token_id);
    let attributes = layers
        .iter()
        .map(|l| Attribute {
            trait_type: l.trait_type.clone(),
            value: l.value.clone(),
        })
        .collect();

    NftMetadata {
        name,
        description,
        image,
        edition: token_id,
        attributes,
    }
}


fn compress_png(path: &str, level: u8) -> anyhow::Result<()> {
    let level = level.min(6);
    let mut options = Options::from_preset(level);
    options.fix_errors = true;

    let p = PathBuf::from(path);
    let in_file = InFile::Path(p.clone());
    let out_file = OutFile::Path {
        path: Some(p),
        preserve_attrs: true,
    };

    oxipng::optimize(&in_file, &out_file, &options)?;
    Ok(())
}

