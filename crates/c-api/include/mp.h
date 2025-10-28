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

enum Mp4TrackKind foo(void);
