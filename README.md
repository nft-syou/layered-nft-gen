# layered-nft-gen

レイヤーを重ねて NFT コレクションを生成する Rust 製ツール

## 特徴

- **レイヤーベース生成**: 複数のレイヤー（背景、キャラクター、アクセサリーなど）を重ねて一意な NFT を生成
- **レア度設定**: 各レイヤーに重み付けを設定し、レアリティを調整可能
- **禁則ルール**: 特定の組み合わせを禁止する制約機能
- **重複防止**: 完全に同じパターンが生成されないことを保証
- **並列処理**: Rayon による高速な並列生成
- **PNG 圧縮**: oxipng による最適化（オプション）
- **OpenSea 互換**: OpenSea 標準のメタデータ JSON を自動生成

## 必要要件

- Rust 1.70 以上

## インストール

```bash
git clone https://github.com/yourusername/layered-nft-gen.git
cd layered-nft-gen
cargo build --release
```

## 使い方

### 1. レイヤー画像の準備

`layers/` ディレクトリに各レイヤーのフォルダを作成し、PNG 画像を配置します：

```
layers/
├── Background/
│   └── Black.png
├── Eyeball/
│   ├── Red.png
│   └── White.png
├── Iris/
│   ├── Large.png
│   ├── Medium.png
│   └── Small.png
└── Eye color/
    ├── Cyan.png
    ├── Green.png
    └── Red.png
```

**重要**: すべてのレイヤー画像は同じサイズ（幅・高さ）にしてください。

### 2. 設定ファイルの編集

`config.yaml` を編集して、生成枚数、レイヤー構成、レア度を設定します：

```yaml
# 生成枚数
count: 100

# 出力ディレクトリ
output:
  image_dir: "output/images"
  metadata_dir: "output/metadata"
  png_compression:
    enabled: true
    level: 4  # 0-6 (高いほど圧縮率が高いが時間がかかる)

# メタデータ設定
metadata:
  name: "Your Collection"
  description: "Your NFT collection description"
  base_image_url: "https://example.com/images"

# レイヤー構成（上から順に重ねられます）
layers:
  - name: "Background"
    directory: "layers/Background"
    rarity:
      "Black.png": 1

  - name: "Eyeball"
    directory: "layers/Eyeball"
    rarity:
      "Red.png": 50
      "White.png": 50

  - name: "Eye color"
    directory: "layers/Eye color"
    rarity:
      "Cyan.png": 1
      "Green.png": 1
      "Red.png": 10  # 他より10倍出やすい

# 禁則ルール（特定の組み合わせを禁止）
constraints:
  forbidden_pairs:
    # 赤い眼球と赤い目の色の組み合わせを禁止
    - a: { trait_type: "Eyeball", value: "Red" }
      b: { trait_type: "Eye color", value: "Red" }
```

### 3. 生成実行

```bash
cargo run --release
```

生成された画像とメタデータは以下に出力されます：

```
output/
├── images/
│   ├── 1.png
│   ├── 2.png
│   └── ...
└── metadata/
    ├── 1.json
    ├── 2.json
    └── ...
```

### 4. 検証（オプション）

生成後、メタデータの統計と禁則ルールの違反をチェックできます：

```bash
cargo run --bin check --release
```

このコマンドは以下を実行します：

- **レア度統計**: 各トレイトの出現率を集計・表示
- **禁則チェック**: `config.yaml` の `forbidden_pairs` に違反がないか検証
- **CI 対応**: 違反があれば exit code 1 で終了

出力例：

```
==============================
 NFT Rarity Check
 Total tokens: 100
==============================

▶ Trait: Eyeball
  White                            52 (52.00%)
  Red                              48 (48.00%)

▶ Trait: Eye color
  Yellow                           65 (65.00%)
  Green                            7 (7.00%)
  ...

==============================
 Forbidden-pairs Check
 Violations(tokens): 0
==============================
✅ 禁則違反は見つかりませんでした
```

## レア度設定のヒント

レア度の数値は**相対的な重み**です：

```yaml
rarity:
  "Common.png": 100     # よく出る（100/110 = 約90.9%）
  "Rare.png": 10        # たまに出る（10/110 = 約9.1%）
  "SuperRare.png": 1    # めったに出ない（1/111 = 約0.9%）
```

## 禁則ルールの例

```yaml
constraints:
  forbidden_pairs:
    # サングラスと閉じた目の組み合わせを禁止
    - a: { trait_type: "Accessory", value: "Sunglasses" }
      b: { trait_type: "Eyes", value: "Closed" }

    # 特定のキャラクターと特定の背景の組み合わせを禁止
    - a: { trait_type: "Character", value: "Ninja" }
      b: { trait_type: "Background", value: "Ocean" }
```

## トラブルシューティング

### 「組み合わせ数を超えています」エラー

生成枚数が理論上の最大組み合わせ数を超えています。以下のいずれかを実施してください：

- `config.yaml` の `count` を減らす
- レイヤーのバリエーションを増やす
- 禁則ルールを減らす

### 「一意なパターンを見つけられませんでした」エラー

組み合わせ数ギリギリの枚数を生成しようとしているか、レア度設定が極端すぎる可能性があります：

- `count` を少し減らす
- レア度の偏りを調整する

### レイヤーのサイズが一致しないエラー

すべてのレイヤー画像を同じサイズ（幅・高さ）にしてください。

## クレジット

このリポジトリのサンプル画像は [HashLips Art Engine](https://github.com/HashLips/hashlips_art_engine) から借用しています。HashLips チームに感謝します。

## 作者

[@nft_syou](https://x.com/nft_syou)

## ライセンス

MIT

## 貢献

Issue や Pull Request を歓迎します！
