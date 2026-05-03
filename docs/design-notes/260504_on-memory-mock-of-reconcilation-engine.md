# DesignDoc: RIST serverのreconcilation機構をon memoryにmockする

Author: @comavius
First published: 2025-05-04
Status: **Draft**

## Review Status

| reviewer | status |
| :---- | :---- |
| A | requested |
| A | requested |
| A | requested |

## Objectives

### Goal
- RISTサーバーそのものとそのreconcilation機構をon memoryにmockする
- その状態で、サーバーから動画のs3へのアップロードができるようにする

### Not Goal

## Background
まずは軽い構成で小さく作りたい。特にreconcilationがうまく動かなかったら困るので、早い段階で問題をあぶり出したい。

## System Overview
RISTサーバーとrecording consoleサーバーを同じプロセス内で動かす。境界はDBのテーブルを模した`Arc<Mutex<State>>`として、チャンネルのような機能は用いない。

## Detailed Design
RISTサーバーの機能を確かめる上でs3が必要なので、docker composeを用いて立ち上げる。必要なコンテナは以下の通り:
- mock recording console server
- s3 (MinIO)

RISTクライアントの詳細は後ほど決める。

## Backward Compatibility


## Release


## Tasks


## Caveats


## Scalability


## Security Concerns / Privacy Concerns


## Alternatives Considered


## References


## Appendix
