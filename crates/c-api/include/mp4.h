#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef enum Mp4TrackKind {
  /**
   * 音声トラック
   */
  Audio = 0,
  /**
   * 映像トラック
   */
  Video = 1,
} Mp4TrackKind;

typedef enum Mp4SampleEntryKind {
  /**
   * Unknown
   */
  Unknown = 0,
  /**
   * AVC1 (H.264)
   */
  Avc1,
  /**
   * HEV1 (H.265/HEVC)
   */
  Hev1,
  /**
   * VP08 (VP8)
   */
  Vp08,
  /**
   * VP09 (VP9)
   */
  Vp09,
  /**
   * AV01 (AV1)
   */
  Av01,
  /**
   * Opus
   */
  Opus,
  /**
   * MP4A (AAC)
   */
  Mp4a,
} Mp4SampleEntryKind;

typedef enum Mp4Error {
  Ok = 0,
  InvalidInput,
  InvalidData,
  InvalidState,
  InputRequired,
  OutputRequired,
  NullPointer,
  NoMoreSamples,
  Other,
} Mp4Error;

typedef struct CodecSpecificData CodecSpecificData;

typedef struct Mp4FileDemuxer Mp4FileDemuxer;

typedef struct Option_CString Option_CString;

typedef struct Option_Mp4FileMuxer Option_Mp4FileMuxer;

typedef struct Vec_Output Vec_Output;

typedef struct Mp4SampleEntry {
  SampleEntry inner;
  struct CodecSpecificData data;
} Mp4SampleEntry;

typedef struct Mp4SampleEntryAvc1 {
  uint32_t width;
  uint32_t height;
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

typedef struct Mp4DemuxTrackInfo {
  uint32_t track_id;
  enum Mp4TrackKind kind;
  uint64_t duration;
  uint32_t timescale;
} Mp4DemuxTrackInfo;

typedef struct Mp4DemuxSample {
  const struct Mp4DemuxTrackInfo *track;
  bool keyframe;
  uint64_t timestamp;
  uint32_t duration;
  uint64_t data_offset;
  uintptr_t data_size;
} Mp4DemuxSample;

typedef struct Mp4FileMuxer {
  Mp4FileMuxerOptions options;
  struct Option_Mp4FileMuxer inner;
  struct Option_CString last_error_string;
  struct Vec_Output output_list;
  uintptr_t next_output_index;
} Mp4FileMuxer;

typedef struct Mp4MuxSample {
  enum Mp4TrackKind track_kind;
  const struct Mp4SampleEntry *sample_entry;
  bool keyframe;
  uint64_t duration_micros;
  uint64_t data_offset;
  uint32_t data_size;
} Mp4MuxSample;

enum Mp4TrackKind foo(void);

enum Mp4SampleEntryKind mp4_sample_entry_get_kind(const struct Mp4SampleEntry *entry);

enum Mp4Error mp4_sample_entry_get_avc1(const struct Mp4SampleEntry *entry,
                                        struct Mp4SampleEntryAvc1 *out_entry);

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
