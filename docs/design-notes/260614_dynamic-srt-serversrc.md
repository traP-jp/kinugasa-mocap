# DesignDoc: Template

Author: 
First published: 
Status: **Draft**

## Review Status

| reviewer | status |
| :---- | :---- |
| A | requested |
| A | requested |
| A | requested |

## Objectives

### Goal
- gstreamerのserversrcとして、SRTを受信できるようなgst-pluginを実装する。
- このserversrcはポートを1つ(あるいは定数個)使用し複数の接続を受け付け、stream IDによるdemuxを行なうことができる。
- 実行中に動的にstream IDを追加・削除できる必要がある。

### Not Goal
- 後述する実行ファイル`srt-recerive-multi-stream`は、あくまでテスト用途であり、主なコンポーネントはすべて`kinugasa_gst::srt`に実装すること。

## Background
- stream ID(あるいは類似の機構)によるdemuxをRIST + moblinで実現することが難しいため、SRTからの入力を受け取ることにした。

## System Overview
stream IDの払い出しは上位のコンポーネントが行なうこととし、このコンポーネントは与えられたstream IDによってlisten, demuxを行いgstreamerのserversrcとして動作することが目的である。

## Security Concerns / Privacy Concerns
- AES-256による暗号化に対応すること。

## Tests
- pluginに対するテストを行うため、`srt-receive-multi-stream`バイナリを`kinugasa-gst`crateから公開すること。このバイナリの役割については`rist_receive`及び`srt_receive`の実装を参照すること。
- `srt-receive-multi-stream`について、以下のテストを行うこと。ただし、すべてのテストはAES-256による暗号化を有効にした状態で行うこと。
    - 単一のstream IDで複数のクライアントから接続し、正しく受信できること。
    - 複数のstream IDで複数のクライアントから1つのサーバーに接続し、正しく受信できること。これは複数の`protocol-e2e`コンテナを同時に起動して行うこと。
    - 実行中にstream IDを追加し、追加したstream IDで接続したクライアントから正しく受信できること。
    - 実行中にstream IDを削除し、削除したstream IDで接続したクライアントからの接続が拒否されること。
