# ドキュメントに関するドキュメント

このドキュメントは、 `kinugasa-mocap/docs` 内のドキュメントについて説明するものです。

## 構成
`kinugasa-mocap/docs` は、以下のような構成になっています。
- `docs/design-notes/`
    - 全デザインノート
- `docs/design-notes-ongoing/`
    - 進行中の作業に関するデザインノートのsymlink集
- `docs/templates/`
    - テンプレート集
- `docs/doc-of-docs.md`
    - この文書

## デザインノート (`docs/design-notes/`, `docs/design-notes-ongoing/`)

デザインノートの使い方についてはこの文書を参照してください: https://speakerdeck.com/munetoshi/how-to-write-a-design-doc-ja-ver-dot
TODO: この speakerdeck を AI から参照しやすいようにテキスト化する。

### 作成時
```bash
nix run .#dev:create-design-note <short-description>
```
作成時は、 `docs/templates/design-note.md` をテンプレートとして使用してください。`docs/design-notes/<yymmdd>_<short-description>.md` にテキストファイルを作成し、 `docs/design-notes-ongoing/<short-description>.md` にsymlinkを張ってください。

### 完了時
```bash
nix run .#dev:complete-design-note <short-description>
```
完了時は、 `docs/design-notes-ongoing/<short-description>.md` のsymlinkを削除してください。
