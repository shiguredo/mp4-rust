/* Generated with cbindgen:0.29.2 */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef enum Mp4Error {
  MP4_ERROR_OK = 0,
  MP4_ERROR_INVALID_INPUT,
  MP4_ERROR_INVALID_DATA,
  MP4_ERROR_INVALID_STATE,
  MP4_ERROR_INPUT_REQUIRED,
  MP4_ERROR_OUTPUT_REQUIRED,
  MP4_ERROR_NULL_POINTER,
  MP4_ERROR_NO_MORE_SAMPLES,
  MP4_ERROR_UNSUPPORTED,
  MP4_ERROR_OTHER,
} Mp4Error;

typedef enum Mp4TrackKind {
  /**
   * 音声トラック
   */
  MP4_TRACK_KIND_AUDIO = 0,
  /**
   * 映像トラック
   */
  MP4_TRACK_KIND_VIDEO = 1,
} Mp4TrackKind;

typedef enum Mp4SampleEntryKind {
  /**
   * AVC1 (H.264)
   */
  MP4_SAMPLE_ENTRY_KIND_AVC1,
  /**
   * HEV1 (H.265/HEVC)
   */
  MP4_SAMPLE_ENTRY_KIND_HEV1,
  /**
   * VP08 (VP8)
   */
  MP4_SAMPLE_ENTRY_KIND_VP08,
  /**
   * VP09 (VP9)
   */
  MP4_SAMPLE_ENTRY_KIND_VP09,
  /**
   * AV01 (AV1)
   */
  MP4_SAMPLE_ENTRY_KIND_AV01,
  /**
   * Opus
   */
  MP4_SAMPLE_ENTRY_KIND_OPUS,
  /**
   * MP4A (AAC)
   */
  MP4_SAMPLE_ENTRY_KIND_MP4A,
} Mp4SampleEntryKind;

typedef struct Option_CString Option_CString;

typedef struct Option_Mp4FileMuxer Option_Mp4FileMuxer;

typedef struct Vec_Output Vec_Output;

typedef struct Mp4DemuxTrackInfo {
  uint32_t track_id;
  enum Mp4TrackKind kind;
  uint64_t duration;
  uint32_t timescale;
} Mp4DemuxTrackInfo;

typedef struct Mp4SampleEntryAvc1 {
  uint16_t width;
  uint16_t height;
  uint8_t avc_profile_indication;
  uint8_t profile_compatibility;
  uint8_t avc_level_indication;
  uint8_t length_size_minus_one;
  const uint8_t *const *sps_data;
  const uint32_t *sps_sizes;
  uint32_t sps_count;
  const uint8_t *const *pps_data;
  const uint32_t *pps_sizes;
  uint32_t pps_count;
  bool is_chroma_format_present;
  uint8_t chroma_format;
  bool is_bit_depth_luma_minus8_present;
  uint8_t bit_depth_luma_minus8;
  bool is_bit_depth_chroma_minus8_present;
  uint8_t bit_depth_chroma_minus8;
} Mp4SampleEntryAvc1;

typedef struct Mp4SampleEntryHev1 {
  uint16_t width;
  uint16_t height;
  uint8_t general_profile_space;
  uint8_t general_tier_flag;
  uint8_t general_profile_idc;
  uint32_t general_profile_compatibility_flags;
  uint64_t general_constraint_indicator_flags;
  uint8_t general_level_idc;
  uint8_t chroma_format_idc;
  uint8_t bit_depth_luma_minus8;
  uint8_t bit_depth_chroma_minus8;
  uint16_t min_spatial_segmentation_idc;
  uint8_t parallelism_type;
  uint16_t avg_frame_rate;
  uint8_t constant_frame_rate;
  uint8_t num_temporal_layers;
  uint8_t temporal_id_nested;
  uint8_t length_size_minus_one;
  uint32_t nalu_array_count;
  const uint8_t *nalu_types;
  const uint32_t *nalu_counts;
  const uint8_t *const *nalu_data;
  const uint32_t *nalu_sizes;
} Mp4SampleEntryHev1;

typedef struct Mp4SampleEntryVp08 {
  uint16_t width;
  uint16_t height;
  uint8_t bit_depth;
  uint8_t chroma_subsampling;
  bool video_full_range_flag;
  uint8_t colour_primaries;
  uint8_t transfer_characteristics;
  uint8_t matrix_coefficients;
} Mp4SampleEntryVp08;

typedef struct Mp4SampleEntryVp09 {
  uint16_t width;
  uint16_t height;
  uint8_t profile;
  uint8_t level;
  uint8_t bit_depth;
  uint8_t chroma_subsampling;
  bool video_full_range_flag;
  uint8_t colour_primaries;
  uint8_t transfer_characteristics;
  uint8_t matrix_coefficients;
  const uint8_t *codec_initialization_data;
  uint32_t codec_initialization_data_size;
} Mp4SampleEntryVp09;

typedef struct Mp4SampleEntryAv01 {
  uint16_t width;
  uint16_t height;
  uint8_t seq_profile;
  uint8_t seq_level_idx_0;
  uint8_t seq_tier_0;
  uint8_t high_bitdepth;
  uint8_t twelve_bit;
  uint8_t monochrome;
  uint8_t chroma_subsampling_x;
  uint8_t chroma_subsampling_y;
  uint8_t chroma_sample_position;
  bool initial_presentation_delay_present;
  uint8_t initial_presentation_delay_minus_one;
  const uint8_t *config_obus;
  uint32_t config_obus_size;
} Mp4SampleEntryAv01;

typedef struct Mp4SampleEntryOpus {
  uint8_t channel_count;
  uint16_t sample_rate;
  uint16_t sample_size;
  uint16_t pre_skip;
  uint32_t input_sample_rate;
  int16_t output_gain;
} Mp4SampleEntryOpus;

typedef struct Mp4SampleEntryMp4a {
  uint8_t channel_count;
  uint16_t sample_rate;
  uint16_t sample_size;
  uint32_t buffer_size_db;
  uint32_t max_bitrate;
  uint32_t avg_bitrate;
  const uint8_t *dec_specific_info;
  uint32_t dec_specific_info_size;
} Mp4SampleEntryMp4a;

typedef union Mp4SampleEntryData {
  struct Mp4SampleEntryAvc1 avc1;
  struct Mp4SampleEntryHev1 hev1;
  struct Mp4SampleEntryVp08 vp08;
  struct Mp4SampleEntryVp09 vp09;
  struct Mp4SampleEntryAv01 av01;
  struct Mp4SampleEntryOpus opus;
  struct Mp4SampleEntryMp4a mp4a;
} Mp4SampleEntryData;

typedef struct Mp4SampleEntry {
  enum Mp4SampleEntryKind kind;
  union Mp4SampleEntryData data;
} Mp4SampleEntry;

typedef struct Mp4DemuxSample {
  const struct Mp4DemuxTrackInfo *track;
  const struct Mp4SampleEntry *sample_entry;
  bool keyframe;
  uint64_t timestamp;
  uint32_t duration;
  uint64_t data_offset;
  uintptr_t data_size;
} Mp4DemuxSample;

typedef struct Mp4MuxSample {
  enum Mp4TrackKind track_kind;
  const struct Mp4SampleEntry *sample_entry;
  bool keyframe;
  uint64_t duration_micros;
  uint64_t data_offset;
  uint32_t data_size;
} Mp4MuxSample;

struct Mp4FileDemuxer *mp4_file_demuxer_new(void);

void mp4_file_demuxer_free(struct Mp4FileDemuxer *demuxer);

const char *mp4_file_demuxer_get_last_error(const struct Mp4FileDemuxer *demuxer);

enum Mp4Error mp4_file_demuxer_get_required_input(struct Mp4FileDemuxer *demuxer,
                                                  uint64_t *out_required_input_position,
                                                  int32_t *out_required_input_size);

enum Mp4Error mp4_file_demuxer_handle_input(struct Mp4FileDemuxer *demuxer,
                                            uint64_t input_position,
                                            const uint8_t *input_data,
                                            uint32_t input_data_size);

enum Mp4Error mp4_file_demuxer_get_tracks(struct Mp4FileDemuxer *demuxer,
                                          const struct Mp4DemuxTrackInfo **out_tracks,
                                          uint32_t *out_track_count);

enum Mp4Error mp4_file_demuxer_next_sample(struct Mp4FileDemuxer *demuxer,
                                           struct Mp4DemuxSample *out_sample);

uint32_t mp4_estimate_maximum_moov_box_size(uint32_t audio_sample_count,
                                            uint32_t video_sample_count);

struct Mp4FileMuxer *mp4_file_muxer_new(void);

void mp4_file_muxer_free(struct Mp4FileMuxer *muxer);

const char *mp4_file_muxer_get_last_error(const struct Mp4FileMuxer *muxer);

enum Mp4Error mp4_file_muxer_set_reserved_moov_box_size(struct Mp4FileMuxer *muxer, uint64_t size);

enum Mp4Error mp4_file_muxer_set_creation_timestamp(struct Mp4FileMuxer *muxer,
                                                    uint64_t timestamp_micros);

enum Mp4Error mp4_file_muxer_initialize(struct Mp4FileMuxer *muxer);

enum Mp4Error mp4_file_muxer_next_output(struct Mp4FileMuxer *muxer,
                                         uint64_t *out_output_offset,
                                         uint32_t *out_output_size,
                                         const uint8_t **out_output_data);

enum Mp4Error mp4_file_muxer_append_sample(struct Mp4FileMuxer *muxer,
                                           const struct Mp4MuxSample *sample);

enum Mp4Error mp4_file_muxer_finalize(struct Mp4FileMuxer *muxer);
