/**
 * Node.js で mp4-rust の WASM を使って MP4 ファイルに無音の Opus を 10 秒分書き込むサンプル
 *
 * # 使用方法
 *
 * ```bash
 * cargo build --release --target wasm32-unknown-unknown -p wasm
 * node crates/wasm/examples/mux.js /path/to/output.mp4
 * ```
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// wasm ファイルを読み込んで初期化する
const wasmPath = path.join(__dirname, '../../../target/wasm32-unknown-unknown/release/mp4_wasm.wasm');
const wasmBuffer = fs.readFileSync(wasmPath);
const wasmInstance = await WebAssembly.instantiate(wasmBuffer);

const {
    memory,
    mp4_file_muxer_new,
    mp4_file_muxer_free,
    mp4_file_muxer_set_reserved_moov_box_size,
    mp4_file_muxer_initialize,
    mp4_file_muxer_append_sample,
    mp4_file_muxer_finalize,
    mp4_file_muxer_next_output,
    mp4_file_muxer_get_last_error,
    mp4_estimate_maximum_moov_box_size,
    mp4_mux_sample_from_json,
    mp4_mux_sample_free,
    mp4_alloc,
    mp4_free,
    mp4_vec_ptr,
    mp4_vec_len,
    mp4_vec_free,
} = wasmInstance.instance.exports;

// メモリバッファへのアクセスを簡略化する関数群
function readCString(ptr) {
    const view = new Uint8Array(memory.buffer, ptr);
    let length = 0;
    while (view[length] !== 0) length++;
    return new TextDecoder().decode(view.slice(0, length));
}

function readU8Array(ptr, len) {
    return new Uint8Array(memory.buffer, ptr, len);
}

function writeU8Array(ptr, data) {
    const view = new Uint8Array(memory.buffer, ptr, data.length);
    view.set(data);
}

function readJSON(vecPtr) {
    const ptr = mp4_vec_ptr(vecPtr);
    const len = mp4_vec_len(vecPtr);
    const bytes = readU8Array(ptr, len);
    const jsonStr = new TextDecoder().decode(bytes);
    mp4_vec_free(vecPtr);
    return JSON.parse(jsonStr);
}

// 無音の Opus フレームデータ（3バイト）
// Opus の無音パケット: 0xf8 0xff 0xfe
const OPUS_SILENCE_FRAME = new Uint8Array([0xf8, 0xff, 0xfe]);

async function createMP4WithOpus(outputPath) {
    console.log(`Creating MP4 file with 10 seconds of silent Opus audio...`);

    const file = fs.openSync(outputPath, 'w');

    try {
        // マルチプレックサーを作成
        const muxerPtr = mp4_file_muxer_new();
        if (!muxerPtr) {
            console.error('Error: Could not create muxer');
            process.exit(1);
        }

        // 推定される moov ボックスサイズを設定
        // 10秒間のOpus（48kHz）= 10秒 * 50フレーム/秒 = 500フレーム
        const estimatedMoovSize = mp4_estimate_maximum_moov_box_size(500, 0);
        console.log(`Estimated moov box size: ${estimatedMoovSize} bytes`);
        mp4_file_muxer_set_reserved_moov_box_size(muxerPtr, estimatedMoovSize);

        // マルチプレックサーを初期化
        let err = mp4_file_muxer_initialize(muxerPtr);
        if (err !== 0) { // MP4_ERROR_OK
            const errorMsg = readCString(mp4_file_muxer_get_last_error(muxerPtr));
            console.error(`Error: Failed to initialize muxer: ${errorMsg}`);
            mp4_file_muxer_free(muxerPtr);
            process.exit(1);
        }

        console.log('Muxer initialized');

        // 初期出力データを取得してファイルに書き込む
        let currentOffset = 0;
        const outputOffsetPtr = mp4_alloc(8);  // uint64_t
        const outputSizePtr = mp4_alloc(4);    // uint32_t
        const outputDataPtrPtr = mp4_alloc(8); // pointer

        console.log('Writing initial boxes...');

        while (true) {
            err = mp4_file_muxer_next_output(muxerPtr, outputOffsetPtr, outputSizePtr, outputDataPtrPtr);
            if (err !== 0) {
                console.error(`Error: Failed to get output: ${err}`);
                break;
            }

            const offsetView = new BigUint64Array(memory.buffer, outputOffsetPtr, 1);
            const sizeView = new Uint32Array(memory.buffer, outputSizePtr, 1);
            const dataPtrView = new BigUint64Array(memory.buffer, outputDataPtrPtr, 1);

            const offset = Number(offsetView[0]);
            const size = sizeView[0];
            const dataPtr = Number(dataPtrView[0]);

            if (size === 0) break;

            const data = readU8Array(dataPtr, size);
            fs.writeSync(file, Buffer.from(data), 0, size, offset);
            console.log(`  Wrote ${size} bytes at offset ${offset}`);

            currentOffset = Math.max(currentOffset, offset + size);
        }

        mp4_free(outputOffsetPtr, 8);
        mp4_free(outputSizePtr, 4);
        mp4_free(outputDataPtrPtr, 8);

        // Opus サンプルエントリーを作成
        const opusEntryJson = {
            kind: 'opus',
            channelCount: 2,
            sampleRate: 48000,
            sampleSize: 16,
            preSkip: 0,
            inputSampleRate: 48000,
            outputGain: 0,
        };

        // 10 秒間のサンプルを追加
        // 48kHz, ステレオ = 48000 サンプル/秒
        // 20ms フレーム = 960 サンプル/フレーム
        // 10秒 = 500フレーム
        const FRAME_DURATION = 960; // 48000 / 1000 * 20ms
        const NUM_FRAMES = 500;     // 10秒 * 50フレーム/秒

        console.log(`Adding ${NUM_FRAMES} Opus frames (${NUM_FRAMES * 20}ms = 10 seconds)...`);

        for (let i = 0; i < NUM_FRAMES; i++) {
            // サンプルデータをファイルに書き込む
            fs.writeSync(file, Buffer.from(OPUS_SILENCE_FRAME), 0, OPUS_SILENCE_FRAME.length, currentOffset);

            // サンプル情報を JSON で構築
            const muxSampleJson = {
                track_kind: 'audio',
                keyframe: true,
                timescale: 48000,
                duration: FRAME_DURATION,
                data_offset: currentOffset,
                data_size: OPUS_SILENCE_FRAME.length,
                sample_entry: opusEntryJson,
            };

            // JSON をシリアライズ
            const jsonStr = JSON.stringify(muxSampleJson);
            const jsonBytes = new TextEncoder().encode(jsonStr);

            // WASM メモリに JSON を書き込む
            const sampleJsonPtr = mp4_alloc(jsonBytes.length);
            writeU8Array(sampleJsonPtr, jsonBytes);

            // JSON を Mp4MuxSample に変換
            const samplePtr = mp4_mux_sample_from_json(sampleJsonPtr, jsonBytes.length);
            if (!samplePtr) {
                const errorMsg = readCString(mp4_file_muxer_get_last_error(muxerPtr));
                console.error(`Error: Failed to convert sample ${i} from JSON: ${errorMsg}`);
                mp4_file_muxer_free(muxerPtr);
                mp4_free(sampleJsonPtr, jsonBytes.length);
                process.exit(1);
            }

            // Mp4MuxSample をマルチプレックサーに追加
            err = mp4_file_muxer_append_sample(muxerPtr, samplePtr);
            if (err !== 0) {
                const errorMsg = readCString(mp4_file_muxer_get_last_error(muxerPtr));
                console.error(`Error: Failed to append sample ${i}: ${errorMsg}`);
                mp4_mux_sample_free(samplePtr);
                mp4_file_muxer_free(muxerPtr);
                mp4_free(sampleJsonPtr, jsonBytes.length);
                process.exit(1);
            }

            // リソースを解放
            mp4_mux_sample_free(samplePtr);
            mp4_free(sampleJsonPtr, jsonBytes.length);

            currentOffset += OPUS_SILENCE_FRAME.length;

            if ((i + 1) % 100 === 0) {
                console.log(`  Added ${i + 1} frames...`);
            }
        }

        console.log(`Total frames added: ${NUM_FRAMES}`);

        // マルチプレックス処理を完了
        console.log('Finalizing muxer...');
        err = mp4_file_muxer_finalize(muxerPtr);
        if (err !== 0) {
            const errorMsg = readCString(mp4_file_muxer_get_last_error(muxerPtr));
            console.error(`Error: Failed to finalize muxer: ${errorMsg}`);
            mp4_file_muxer_free(muxerPtr);
            process.exit(1);
        }

        // ファイナライズ後の出力データをファイルに書き込む
        console.log('Writing final boxes...');

        const outputOffsetPtr2 = mp4_alloc(8);
        const outputSizePtr2 = mp4_alloc(4);
        const outputDataPtrPtr2 = mp4_alloc(8);

        while (true) {
            err = mp4_file_muxer_next_output(muxerPtr, outputOffsetPtr2, outputSizePtr2, outputDataPtrPtr2);
            if (err !== 0) {
                console.error(`Error: Failed to get final output: ${err}`);
                break;
            }

            const offsetView = new BigUint64Array(memory.buffer, outputOffsetPtr2, 1);
            const sizeView = new Uint32Array(memory.buffer, outputSizePtr2, 1);
            const dataPtrView = new BigUint64Array(memory.buffer, outputDataPtrPtr2, 1);

            const offset = Number(offsetView[0]);
            const size = sizeView[0];
            const dataPtr = Number(dataPtrView[0]);

            if (size === 0) break;

            const data = readU8Array(dataPtr, size);
            fs.writeSync(file, Buffer.from(data), 0, size, offset);
            console.log(`  Wrote ${size} bytes at offset ${offset}`);
        }

        mp4_free(outputOffsetPtr2, 8);
        mp4_free(outputSizePtr2, 4);
        mp4_free(outputDataPtrPtr2, 8);

        // リソースを解放
        mp4_file_muxer_free(muxerPtr);

        console.log(`\nSuccessfully created '${outputPath}'`);

    } finally {
        fs.closeSync(file);
    }
}

// メイン処理
const args = process.argv.slice(2);
if (args.length === 0) {
    console.error('Usage: node mux.js <output_mp4>');
    process.exit(1);
}

const outputPath = args[0];
await createMP4WithOpus(outputPath);

