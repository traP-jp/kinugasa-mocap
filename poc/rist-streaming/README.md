# RIST Streaming PoC

`docs/design-notes-ongoing/rist-poc.md` に沿って、RIST 受信、録画ストレージへのアップロード、LiveKit 経由の配信を最小構成で検証するための Docker Compose 構成です。

## Components

- `livekit`: LiveKit Server
- `ingress`: LiveKit Ingress。RTMP を受けて LiveKit room に publish します
- `rist-server`: ffmpeg で RIST を受け、LiveKit Ingress と録画セグメントへ出力します。録画は HLS muxer の `temp_file` で、完成済み `.ts` だけが見える形にします。終了時は ffmpeg に `q` を送り、HLS playlist を閉じます
- `recording-uploader`: サイズが安定した録画セグメントと `index.m3u8` を MinIO にアップロードします。`rist-server` とは独立に動き、終了時もしばらく最終アップロードを試みます
- `minio`: S3 互換 Recording Storage
- `viewer`: LiveKit の映像確認用の軽量 WebUI
- `lk`: LiveKit CLI コンテナ

## Setup

```sh
cd poc/rist-streaming
cp .env.example .env
docker compose up -d redis livekit ingress minio minio-init viewer
./scripts/create-ingress.sh
```

`create-ingress.sh` の最後に出る `LIVEKIT_RTMP_URL=...` を `.env` に書き込むか、同じ shell で `export` してください。

## Synthetic Camera Test

カメラ実機の前に ffmpeg の `testsrc2` を RIST 送信元として使います。
RIST クライアントは既定で `libx264` を使い、H.264 baseline/level 3.1 の MPEG-TS を RIST に載せます。

```sh
./scripts/run-testsrc.sh
```

別 terminal で視聴 token を作ります。

```sh
./scripts/token.sh viewer
```

`http://localhost:3000` を開き、LiveKit URL に `ws://localhost:7880`、Token に生成された token を入れると配信映像を確認できます。3000 番が埋まっている場合は `VIEWER_PORT=3001 docker compose up -d viewer` のように変更できます。

MinIO Console は `http://localhost:9001` です。ログインは `.env` の `MINIO_ROOT_USER` と `MINIO_ROOT_PASSWORD` を使います。録画オブジェクトの確認だけなら次でも見られます。

```sh
./scripts/list-recordings.sh
```

stream 系サービスを止めると、MinIO の `recordings/<ROOM_NAME>/index.m3u8` に再生可能な HLS playlist が残ります。`#EXT-X-ENDLIST` が付いているので、prefix ごと取得すれば `ffplay index.m3u8` で確認できます。

HLS 一式を MinIO からダウンロードして MP4 にまとめる場合は次を使います。

```sh
ROOM_NAME=rist-poc ./scripts/download-mp4.sh
```

出力先は既定で `./out/<ROOM_NAME>.mp4` です。変更する場合は `OUTPUT_DIR` と `OUTPUT_MP4` を指定します。

## Real Camera Test

Linux の V4L2 カメラを使う場合は、`.env` の `CAMERA_DEVICE` などを調整してから実行します。

```sh
docker compose --profile stream --profile camera-v4l2 up rist-server recording-uploader camera-v4l2
```

macOS や Windows のホストカメラを Docker から直接触る構成は未検証です。その場合はホスト側 ffmpeg や OBS から RIST を送るほうが検証しやすいです。

## Verification Checklist

- `rist-server` のログで RIST 入力が認識されている
- `ingress` のログで RTMP 接続と room publish が発生している
- viewer で `rist-bridge` の映像が見える
- MinIO の `recordings/rist-poc/` に `.ts` セグメントが増える
- MinIO の `.ts` が 10 秒前後の長さで `ffprobe` / `ffplay` できる
- `docker compose ... stop` / `down` 後に MinIO の `recordings/rist-poc/index.m3u8` が `#EXT-X-ENDLIST` 付きで残る

## Notes

- LiveKit 公式ドキュメントでは、自己ホスト環境で外部 RTMP/WHIP 入力を LiveKit room に入れるには `livekit/ingress` を別途動かす構成になっています。
- ffmpeg の RIST 入力は公式 ffmpeg protocols documentation の `rist` protocol に依存します。`./scripts/check.sh` はコンテナ内 ffmpeg が `rist`、`flv`、`hls`、`libx264` を持つことを確認します。
- RTMP Ingress は VP8/Opus にトランスコードする設定にしています。H.264 のままだと、Firefox など一部環境で `codec is not supported by remote` となり、viewer が `Subscribed` にならないことがあります。
- 書き込み中の `.ts` を MinIO にコピーすると再生できない断片になることがあるため、録画は `.tmp` から完成時に `.ts` へ rename されるようにし、アップローダもファイルサイズが安定した `.ts` だけを `mc cp` します。ストリーム停止直後の最後のセグメントは短くなることがあります。
- 停止時の最終アップロード猶予は `UPLOAD_SHUTDOWN_GRACE` で調整できます。
- この PoC は本番実装ではなく、録画ファイル形式、保存パス、認証情報、TLS、ネットワーク公開設定はいずれも検証用です。
