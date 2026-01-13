/**
 * Node.js で mp4-rust の WASM を使って MP4 ファイルをリマルチプレックスする例
 *
 * # 使用方法
 *
 * ```bash
 * cargo build --release --target wasm32-unknown-unknown -p wasm
 * node --experimental-wasm-modules crates/wasm/examples/remux.js /path/to/input.mp4 /path/to/output.mp4
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
    mp4_file_demuxer_new,
    mp4_file_demuxer_free,
    mp4_file_demuxer_get_required_input,
    mp4_file_demuxer_handle_input,
    mp4_file_demuxer_get_tracks,
    mp4_file_demuxer_next_sample,
    mp4_file_demuxer_get_last_error,
    mp4_file_muxer_new,
    mp4_file_muxer_free,
    mp4_file_muxer_initialize,
    mp4_file_muxer_next_output,
    mp4_file_muxer_append_sample,
    mp4_file_muxer_finalize,
    mp4_file_muxer_get_last_error: mp4_file_muxer_get_last_error_func,
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

function readJSON(vecPtr) {
    const ptr = mp4_vec_ptr(vecPtr);
    const len = mp4_vec_len(vecPtr);
    const bytes = readU8Array(ptr, len);
    const jsonStr = new TextDecoder().decode(bytes);
    mp4_vec_free(vecPtr);
    return JSON.parse(jsonStr);
}

function writeToMemory(data) {
    const ptr = mp4_alloc(data.length);
    const view = new Uint8Array(memory.buffer, ptr, data.length);
    view.set(data);
    return ptr;
}

async function remuxMP4File(inputPath, outputPath) {
    if (!fs.existsSync(inputPath)) {
        console.error(`Error: File not found: ${inputPath}`);
        process.exit(1);
    }

    const inputSize = fs.statSync(inputPath).size;
    const inputFile = fs.openSync(inputPath, 'r');
    const outputFile = fs.openSync(outputPath, 'w');

    // デマルチプレックサーを作成
    const demuxerPtr = mp4_file_demuxer_new();
    if (!demuxerPtr) {
        console.error('Error: Could not create demuxer');
        process.exit(1);
    }

    // マルチプレックサーを作成
    const muxerPtr = mp4_file_muxer_new();
    if (!muxerPtr) {
        console.error('Error: Could not create muxer');
        mp4_file_demuxer_free(demuxerPtr);
        fs.closeSync(inputFile);
        fs.closeSync(outputFile);
        process.exit(1);
    }

    console.log(`Remuxing ${path.basename(inputPath)}...`);

    // ==================== デマルチプレックス初期化 ====================

    // ファイルデータを読み込んでデマルチプレックス処理を進める
    while (true) {
        const requiredPosPtr = mp4_alloc(8);
        const requiredSizePtr = mp4_alloc(4);

        const err = mp4_file_demuxer_get_required_input(
            demuxerPtr,
            requiredPosPtr,
            requiredSizePtr
        );

        if (err !== 0) {
            const errorMsg = readCString(mp4_file_demuxer_get_last_error(demuxerPtr));
            console.error(`Error: Failed to get required input: ${errorMsg}`);
            mp4_file_demuxer_free(demuxerPtr);
            mp4_file_muxer_free(muxerPtr);
            fs.closeSync(inputFile);
            fs.closeSync(outputFile);
            process.exit(1);
        }

        const requiredPosView = new BigUint64Array(memory.buffer, requiredPosPtr, 1);
        const requiredSizeView = new Int32Array(memory.buffer, requiredSizePtr, 1);

        const requiredPos = requiredPosView[0];
        let requiredSize = requiredSizeView[0];

        mp4_free(requiredPosPtr, 8);
        mp4_free(requiredSizePtr, 4);

        if (requiredSize === 0) {
            break;
        }

        let readSize = requiredSize;
        if (requiredSize === -1) {
            readSize = inputSize - Number(requiredPos);
        }

        const buffer = Buffer.alloc(readSize);
        fs.readSync(inputFile, buffer, 0, readSize, Number(requiredPos));

        const wasmBuffer = mp4_alloc(readSize);
        const wasmView = new Uint8Array(memory.buffer, wasmBuffer, readSize);
        wasmView.set(buffer);

        mp4_file_demuxer_handle_input(
            demuxerPtr,
            requiredPos,
            wasmBuffer,
            readSize
        );

        mp4_free(wasmBuffer, readSize);
    }

    // トラック情報を取得
    const tracksPtr = mp4_alloc(8);
    const trackCountPtr = mp4_alloc(4);

    let tracksErr = mp4_file_demuxer_get_tracks(
        demuxerPtr,
        tracksPtr,
        trackCountPtr
    );

    if (tracksErr !== 0) {
        const errorMsg = readCString(mp4_file_demuxer_get_last_error(demuxerPtr));
        console.error(`Error: Failed to get tracks: ${errorMsg}`);
        mp4_file_demuxer_free(demuxerPtr);
        mp4_file_muxer_free(muxerPtr);
        fs.closeSync(inputFile);
        fs.closeSync(outputFile);
        process.exit(1);
    }

    const trackCountView = new Uint32Array(memory.buffer, trackCountPtr, 1);
    const trackCount = trackCountView[0];

    console.log(`Found ${trackCount} track(s)\n`);

    mp4_free(tracksPtr, 8);
    mp4_free(trackCountPtr, 4);

    // ==================== マルチプレックス初期化 ====================

    let err = mp4_file_muxer_initialize(muxerPtr);
    if (err !== 0) {
        const errorMsg = readCString(mp4_file_muxer_get_last_error_func(muxerPtr));
        console.error(`Error: Failed to initialize muxer: ${errorMsg}`);
        mp4_file_demuxer_free(demuxerPtr);
        mp4_file_muxer_free(muxerPtr);
        fs.closeSync(inputFile);
        fs.closeSync(outputFile);
        process.exit(1);
    }

    // マルチプレックサーの初期出力データを書き込む
    let currentOutputOffset = 0;
    const outputOffsetPtr = mp4_alloc(8);
    const outputSizePtr = mp4_alloc(4);
    const outputDataPtrPtr = mp4_alloc(8);

    console.log('Writing initial muxer boxes...');

    while (true) {
        err = mp4_file_muxer_next_output(
            muxerPtr,
            outputOffsetPtr,
            outputSizePtr,
            outputDataPtrPtr
        );

        if (err !== 0) break;

        const offsetView = new BigUint64Array(memory.buffer, outputOffsetPtr, 1);
        const sizeView = new Uint32Array(memory.buffer, outputSizePtr, 1);
        const dataPtrView = new BigUint64Array(memory.buffer, outputDataPtrPtr, 1);

        const offset = offsetView[0];
        const size = sizeView[0];
        const dataPtr = Number(dataPtrView[0]);

        if (size === 0) break;

        const outputData = readU8Array(dataPtr, size);
        fs.writeSync(outputFile, outputData, 0, size, Number(offset));

        console.log(`  Wrote ${size} bytes at offset ${offset}`);
        currentOutputOffset = Number(offset) + size;
    }

    console.log(`Sample data will start at offset: ${currentOutputOffset}\n`);

    mp4_free(outputOffsetPtr, 8);
    mp4_free(outputSizePtr, 4);
    mp4_free(outputDataPtrPtr, 8);

    // ==================== サンプルのリマルチプレックス ====================

    console.log('Remuxing samples...');

    const samplePtr = mp4_alloc(256);
    let sampleCount = 0;

    while (true) {
        const demuxErr = mp4_file_demuxer_next_sample(demuxerPtr, samplePtr);

        if (demuxErr === 8) { // MP4_ERROR_NO_MORE_SAMPLES
            break;
        }

        if (demuxErr !== 0) {
            const errorMsg = readCString(mp4_file_demuxer_get_last_error(demuxerPtr));
            console.error(`Error: Failed to get next sample: ${errorMsg}`);
            break;
        }

        // サンプルデータを読み込む
        const sampleView = new DataView(memory.buffer, samplePtr, 256);
        const dataOffset = Number(sampleView.getBigUint64(24, true));
        const dataSize = sampleView.getUint32(32, true);

        const sampleData = Buffer.alloc(dataSize);
        fs.readSync(inputFile, sampleData, 0, dataSize, Number(dataOffset));

        // 出力ファイルにサンプルデータを追記
        fs.writeSync(outputFile, sampleData, 0, dataSize, currentOutputOffset);

        // JSON からマルチプレックスサンプルを構築
        // （デマルチプレックスサンプルをマルチプレックスサンプルに変換）
        const muxSampleObj = {
            track_kind: 'video', // TODO: 実際にはトラック情報から取得
            sample_entry: null,
            keyframe: true,
            timescale: 30,
            duration: 1,
            data_offset: currentOutputOffset,
            data_size: dataSize,
        };

        const jsonStr = JSON.stringify(muxSampleObj);
        const jsonBytes = new TextEncoder().encode(jsonStr);
        const jsonPtr = writeToMemory(jsonBytes);

        const muxSamplePtr = mp4_mux_sample_from_json(jsonPtr, jsonBytes.length);
        mp4_free(jsonPtr, jsonBytes.length);

        if (muxSamplePtr === 0) {
            console.error('Error: Failed to create mux sample from JSON');
            break;
        }

        // マルチプレックサーにサンプルを追加
        const appendErr = mp4_file_muxer_append_sample(muxerPtr, muxSamplePtr);
        mp4_mux_sample_free(muxSamplePtr);

        if (appendErr !== 0) {
            const errorMsg = readCString(mp4_file_muxer_get_last_error_func(muxerPtr));
            console.error(`Error: Failed to append sample: ${errorMsg}`);
            break;
        }

        sampleCount++;
        currentOutputOffset += dataSize;

        if (sampleCount % 100 === 0) {
            console.log(`  Processed ${sampleCount} samples`);
        }
    }

    console.log(`Total samples processed: ${sampleCount}\n`);

    mp4_free(samplePtr, 256);

    // ==================== マルチプレックス終了処理 ====================

    console.log('Finalizing muxer...');

    err = mp4_file_muxer_finalize(muxerPtr);
    if (err !== 0) {
        const errorMsg = readCString(mp4_file_muxer_get_last_error_func(muxerPtr));
        console.error(`Error: Failed to finalize muxer: ${errorMsg}`);
        mp4_file_demuxer_free(demuxerPtr);
        mp4_file_muxer_free(muxerPtr);
        fs.closeSync(inputFile);
        fs.closeSync(outputFile);
        process.exit(1);
    }

    // マルチプレックサーの最終出力データを書き込む
    const finalOutputOffsetPtr = mp4_alloc(8);
    const finalOutputSizePtr = mp4_alloc(4);
    const finalOutputDataPtrPtr = mp4_alloc(8);

    while (true) {
        err = mp4_file_muxer_next_output(
            muxerPtr,
            finalOutputOffsetPtr,
            finalOutputSizePtr,
            finalOutputDataPtrPtr
        );

        if (err !== 0) break;

        const offsetView = new BigUint64Array(memory.buffer, finalOutputOffsetPtr, 1);
        const sizeView = new Uint32Array(memory.buffer, finalOutputSizePtr, 1);
        const dataPtrView = new BigUint64Array(memory.buffer, finalOutputDataPtrPtr, 1);

        const offset = offsetView[0];
        const size = sizeView[0];
        const dataPtr = Number(dataPtrView[0]);

        if (size === 0) break;

        const outputData = readU8Array(dataPtr, size);
        fs.writeSync(outputFile, outputData, 0, size, Number(offset));

        console.log(`  Wrote final ${size} bytes at offset ${offset}`);
    }

    mp4_free(finalOutputOffsetPtr, 8);
    mp4_free(finalOutputSizePtr, 4);
    mp4_free(finalOutputDataPtrPtr, 8);

    console.log(`\nSuccessfully remuxed '${path.basename(inputPath)}' to '${path.basename(outputPath)}'`);

    // リソースを解放
    mp4_file_muxer_free(muxerPtr);
    mp4_file_demuxer_free(demuxerPtr);
    fs.closeSync(inputFile);
    fs.closeSync(outputFile);
}

// メイン処理
const args = process.argv.slice(2);
if (args.length < 2) {
    console.error('Usage: node remux.js <input.mp4> <output.mp4>');
    process.exit(1);
}

const inputPath = args[0];
const outputPath = args[1];
await remuxMP4File(inputPath, outputPath);

