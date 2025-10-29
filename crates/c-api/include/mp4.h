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

typedef enum Mp4Error {
  Ok = 0,
  InvalidInput,
  InvalidData,
  InvalidState,
  InputRequired,
  NullPointer,
  NoMoreSamples,
  Other,
} Mp4Error;

typedef struct Mp4FileDemuxer Mp4FileDemuxer;

typedef struct Option_CString Option_CString;

typedef struct Option_Mp4FileMuxer Option_Mp4FileMuxer;

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
} Mp4FileMuxer;

typedef struct Mp4SampleEntry {
  uint8_t _opaque[0];
} Mp4SampleEntry;

typedef struct Mp4MuxSample {
  enum Mp4TrackKind track_kind;
  const struct Mp4SampleEntry *sample_entry;
  bool keyframe;
  uint64_t duration_micros;
  uint64_t data_offset;
  uint32_t data_size;
} Mp4MuxSample;

enum Mp4TrackKind foo(void);

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

enum Mp4Error mp4_file_muxer_get_initial_boxes_bytes(struct Mp4FileMuxer *muxer,
                                                     const uint8_t **out_bytes,
                                                     uint32_t *out_size);

enum Mp4Error mp4_file_muxer_append_sample(struct Mp4FileMuxer *muxer,
                                           const struct Mp4MuxSample *sample);

enum Mp4Error mp4_file_muxer_finalize(struct Mp4FileMuxer *muxer);
