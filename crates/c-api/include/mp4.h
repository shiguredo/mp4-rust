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

typedef struct Mp4TrackInfo {
  uint32_t track_id;
  enum Mp4TrackKind kind;
  uint64_t duration;
  uint32_t timescale;
} Mp4TrackInfo;

typedef struct Mp4Sample {
  const struct Mp4TrackInfo *track;
  bool keyframe;
  uint64_t timestamp;
  uint32_t duration;
  uint64_t data_offset;
  uintptr_t data_size;
} Mp4Sample;

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
                                          const struct Mp4TrackInfo **out_tracks,
                                          uint32_t *out_track_count);

enum Mp4Error mp4_file_demuxer_next_sample(struct Mp4FileDemuxer *demuxer,
                                           struct Mp4Sample *out_sample);
