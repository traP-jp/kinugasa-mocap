# DesignDoc: RISTを中心にしたストリーミングの技術検証

Author: @comavius
First published: 2026-04-30
Status: **Draft**

## Review Status

| reviewer | status |
| :---- | :---- |
| A | requested |
| A | requested |
| A | requested |

## Objectives

`docs/architecture/overview.puml` の記述の通り録画できそうか技術検証をする。

### Goal
以下の要素が正しく動くことを確認する
- カメラでの撮影
- Recording Storage へのアップロード
- LiveKit 越しの動画配信

### Not Goal
- RIST Serverのアプリケーション側の実装
- 本番用のコーディング
- 性能測定
- 具体的なファイル形式や保存先など

## Background

RISTを使ったストリーミングに関する技術検証が中途半端なので、本実装に入る前に怪しい部分をすべて確かめたい。

## System Overview

Docker Compose を使って各サーバーを最小構成で立てて検証する。

## Detailed Design

必要なコンテナは以下の通り:
- LiveKit Server
    - 操作は `lk` コマンドから行う
- LiveKit から配信された映像を確認できる WebUI
    - LiveKit Meet
- RIST WebRTCの詰め替えと録画をするサーバー
    - ffmpeg を使用
- S3互換ストレージとそのコンソール
    - MinIO

`poc/rist-streaming` ブランチで作業し、作業後は `main` 側にマージせず放置する。

## Tasks


## Alternatives Considered


## References


## Appendix
