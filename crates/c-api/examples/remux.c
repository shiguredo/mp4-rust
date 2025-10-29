#include "mp4.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define BUFFER_SIZE (1024 * 1024)  // 1MB buffer size

const char *get_sample_entry_kind_name(enum Mp4SampleEntryKind kind) {
    switch (kind) {
        case MP4_SAMPLE_ENTRY_KIND_AVC1:
            return "AVC1 (H.264)";
        case MP4_SAMPLE_ENTRY_KIND_HEV1:
            return "HEV1 (H.265/HEVC)";
        case MP4_SAMPLE_ENTRY_KIND_VP08:
            return "VP08 (VP8)";
        case MP4_SAMPLE_ENTRY_KIND_VP09:
            return "VP09 (VP9)";
        case MP4_SAMPLE_ENTRY_KIND_AV01:
            return "AV01 (AV1)";
        case MP4_SAMPLE_ENTRY_KIND_OPUS:
            return "Opus";
        case MP4_SAMPLE_ENTRY_KIND_MP4A:
            return "MP4A (AAC)";
        default:
            return "Unknown";
    }
}

int main(int argc, char *argv[]) {
    if (argc < 3) {
        fprintf(stderr, "Usage: %s <input_mp4> <output_mp4>\n", argv[0]);
        return 1;
    }

    const char *input_filepath = argv[1];
    const char *output_filepath = argv[2];

    // ==================== DEMUXER SETUP ====================
    FILE *input_file = fopen(input_filepath, "rb");
    if (!input_file) {
        fprintf(stderr, "Error: Could not open input file '%s'\n", input_filepath);
        return 1;
    }

    struct Mp4FileDemuxer *demuxer = mp4_file_demuxer_new();
    if (!demuxer) {
        fprintf(stderr, "Error: Could not create demuxer\n");
        fclose(input_file);
        return 1;
    }

    uint8_t *read_buffer = (uint8_t *)malloc(BUFFER_SIZE);
    if (!read_buffer) {
        fprintf(stderr, "Error: Could not allocate read buffer\n");
        mp4_file_demuxer_free(demuxer);
        fclose(input_file);
        return 1;
    }

    // ==================== MUXER SETUP ====================
    FILE *output_file = fopen(output_filepath, "wb");
    if (!output_file) {
        fprintf(stderr, "Error: Could not open output file '%s'\n", output_filepath);
        free(read_buffer);
        mp4_file_demuxer_free(demuxer);
        fclose(input_file);
        return 1;
    }

    struct Mp4FileMuxer *muxer = mp4_file_muxer_new();
    if (!muxer) {
        fprintf(stderr, "Error: Could not create muxer\n");
        fclose(output_file);
        free(read_buffer);
        mp4_file_demuxer_free(demuxer);
        fclose(input_file);
        return 1;
    }

    // Initialize muxer
    enum Mp4Error err = mp4_file_muxer_initialize(muxer);
    if (err != MP4_ERROR_OK) {
        fprintf(stderr, "Error: Failed to initialize muxer: %d\n", err);
        const char *error_msg = mp4_file_muxer_get_last_error(muxer);
        if (error_msg) {
            fprintf(stderr, "  %s\n", error_msg);
        }
        mp4_file_muxer_free(muxer);
        fclose(output_file);
        free(read_buffer);
        mp4_file_demuxer_free(demuxer);
        fclose(input_file);
        return 1;
    }

    // Write initial muxer output
    uint64_t output_offset;
    uint32_t output_size;
    const uint8_t *output_data;

    while (mp4_file_muxer_next_output(muxer, &output_offset, &output_size, &output_data) ==
           MP4_ERROR_OK) {
        if (output_size == 0) break;

        if (fseek(output_file, output_offset, SEEK_SET) != 0) {
            fprintf(stderr, "Error: Failed to seek in output file\n");
            goto cleanup;
        }

        if (fwrite(output_data, 1, output_size, output_file) != output_size) {
            fprintf(stderr, "Error: Failed to write to output file\n");
            goto cleanup;
        }

        printf("Wrote %u bytes at offset %lu\n", output_size, output_offset);
    }

    // ==================== DEMUX INPUT ====================
    printf("\nDemuxing input file...\n");

    while (true) {
        uint64_t required_position;
        int32_t required_size;

        err = mp4_file_demuxer_get_required_input(demuxer, &required_position, &required_size);
        if (err != MP4_ERROR_OK) {
            fprintf(stderr, "Error: Failed to get required input: %d\n", err);
            goto cleanup;
        }

        if (required_size == 0) {
            break;
        }

        if (fseek(input_file, required_position, SEEK_SET) != 0) {
            fprintf(stderr, "Error: Could not seek to position %lu\n", required_position);
            goto cleanup;
        }

        size_t read_size = BUFFER_SIZE;
        if (required_size > 0) {
            read_size = (size_t)required_size;
        }

        size_t bytes_read = fread(read_buffer, 1, read_size, input_file);
        if (bytes_read == 0 && ferror(input_file)) {
            fprintf(stderr, "Error: Failed to read input file\n");
            goto cleanup;
        }

        mp4_file_demuxer_handle_input(demuxer, required_position, read_buffer,
                                      (uint32_t)bytes_read);
    }

    // Get track information
    const struct Mp4DemuxTrackInfo *tracks;
    uint32_t track_count;

    err = mp4_file_demuxer_get_tracks(demuxer, &tracks, &track_count);
    if (err != MP4_ERROR_OK) {
        fprintf(stderr, "Error: Failed to get tracks: %d\n", err);
        goto cleanup;
    }

    printf("Found %u track(s)\n\n", track_count);

    // ==================== REMUX SAMPLES ====================
    printf("Remuxing samples...\n");

    uint32_t sample_count = 0;
    struct Mp4DemuxSample demux_sample;

    while (true) {
        err = mp4_file_demuxer_next_sample(demuxer, &demux_sample);

        if (err == MP4_ERROR_NO_MORE_SAMPLES) {
            break;
        }

        if (err != MP4_ERROR_OK) {
            fprintf(stderr, "Error: Failed to get next sample: %d\n", err);
            const char *error_msg = mp4_file_demuxer_get_last_error(demuxer);
            if (error_msg) {
                fprintf(stderr, "  %s\n", error_msg);
            }
            goto cleanup;
        }

        // Create mux sample from demux sample
        struct Mp4MuxSample mux_sample = {
            .track_kind = demux_sample.track->kind,
            .sample_entry = demux_sample.sample_entry,
            .keyframe = demux_sample.keyframe,
            .duration_micros = (uint64_t)demux_sample.duration * 1000000 /
                               demux_sample.track->timescale,
            .data_offset = demux_sample.data_offset,
            .data_size = (uint32_t)demux_sample.data_size,
        };

        // Append sample to muxer
        err = mp4_file_muxer_append_sample(muxer, &mux_sample);
        if (err != MP4_ERROR_OK) {
            fprintf(stderr, "Error: Failed to append sample: %d\n", err);
            const char *error_msg = mp4_file_muxer_get_last_error(muxer);
            if (error_msg) {
                fprintf(stderr, "  %s\n", error_msg);
            }
            goto cleanup;
        }

        sample_count++;

        if (sample_count % 100 == 0) {
            printf("  Processed %u samples\n", sample_count);
        }

        // Write any pending muxer output
        while (mp4_file_muxer_next_output(muxer, &output_offset, &output_size, &output_data) ==
               MP4_ERROR_OK) {
            if (output_size == 0) break;

            if (fseek(output_file, output_offset, SEEK_SET) != 0) {
                fprintf(stderr, "Error: Failed to seek in output file\n");
                goto cleanup;
            }

            if (fwrite(output_data, 1, output_size, output_file) != output_size) {
                fprintf(stderr, "Error: Failed to write to output file\n");
                goto cleanup;
            }
        }
    }

    printf("Total samples processed: %u\n\n", sample_count);

    // ==================== FINALIZE MUXER ====================
    printf("Finalizing muxer...\n");

    err = mp4_file_muxer_finalize(muxer);
    if (err != MP4_ERROR_OK) {
        fprintf(stderr, "Error: Failed to finalize muxer: %d\n", err);
        const char *error_msg = mp4_file_muxer_get_last_error(muxer);
        if (error_msg) {
            fprintf(stderr, "  %s\n", error_msg);
        }
        goto cleanup;
    }

    // Write final muxer output
    while (mp4_file_muxer_next_output(muxer, &output_offset, &output_size, &output_data) ==
           MP4_ERROR_OK) {
        if (output_size == 0) break;

        if (fseek(output_file, output_offset, SEEK_SET) != 0) {
            fprintf(stderr, "Error: Failed to seek in output file\n");
            goto cleanup;
        }

        if (fwrite(output_data, 1, output_size, output_file) != output_size) {
            fprintf(stderr, "Error: Failed to write to output file\n");
            goto cleanup;
        }

        printf("Wrote final %u bytes at offset %lu\n", output_size, output_offset);
    }

    printf("\nSuccessfully remuxed '%s' to '%s'\n", input_filepath, output_filepath);

cleanup:
    mp4_file_muxer_free(muxer);
    fclose(output_file);
    free(read_buffer);
    mp4_file_demuxer_free(demuxer);
    fclose(input_file);

    return 0;
}

