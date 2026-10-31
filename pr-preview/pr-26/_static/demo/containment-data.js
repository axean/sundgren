window.sundgrenDemoContainment =
{
 "target": "dev",
 "base": "main",
 "results": [
  {
   "branch": "frame-crc",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "782fe6c31f5ef468a4c60ceb79c94b8f62a8dd85",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "frame-crc",
     "oid": "01be0d300f3d451570c511692ebe1c245da86301",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 1,
     "files": 3,
     "lines": 33,
     "comment_lines": 6,
     "files_read": 3
    },
    "b": {
     "rev": "dev",
     "oid": "e82b73da827cd6247c98f1678508d909232a930f",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 5,
     "files": 9,
     "lines": 122,
     "comment_lines": 14,
     "files_read": 9
    },
    "a_in_b": {
     "lines": 33,
     "exact": 33,
     "fuzzy": 0,
     "in_order": 33,
     "containment": 1.0,
     "exact_containment": 1.0,
     "ordered_containment": 1.0,
     "files": [
      {
       "path": "rust/src/crc.rs",
       "lines": 14,
       "exact": 14,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 17,
       "exact": 17,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 2,
       "exact": 2,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "rust/src/crc.rs",
       "item": "POLY",
       "kind": "const",
       "start": 6,
       "end": 6,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "POLY",
        "start": 6,
        "end": 6,
        "lines": 1
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "crc16",
       "kind": "function",
       "start": 8,
       "end": 18,
       "lines": 7,
       "exact": 7,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "crc16",
        "start": 8,
        "end": 18,
        "lines": 7
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests",
       "kind": "module",
       "start": 20,
       "end": 28,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "tests",
        "start": 20,
        "end": 28,
        "lines": 3
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests::check_value",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "tests::check_value",
        "start": 24,
        "end": 27,
        "lines": 3
       }
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "CRC_LEN",
       "kind": "const",
       "start": 10,
       "end": 10,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "CRC_LEN",
        "start": 10,
        "end": 10,
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "FrameError",
       "kind": "enum",
       "start": 18,
       "end": 23,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "FrameError",
        "start": 18,
        "end": 23,
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "fmt::Display for FrameError::fmt",
       "kind": "function",
       "start": 26,
       "end": 36,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "fmt::Display for FrameError::fmt",
        "start": 26,
        "end": 36,
        "lines": 2
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "parse",
       "kind": "function",
       "start": 41,
       "end": 57,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "parse",
        "start": 41,
        "end": 57,
        "lines": 3
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "check_crc",
       "kind": "function",
       "start": 59,
       "end": 71,
       "lines": 9,
       "exact": 9,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "check_crc",
        "start": 59,
        "end": 71,
        "lines": 9
       }
      },
      {
       "path": "rust/src/lib.rs",
       "item": "crc",
       "kind": "module",
       "start": 4,
       "end": 4,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/lib.rs",
        "item": "crc",
        "start": 4,
        "end": 4,
        "lines": 1
       }
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/lib.rs",
        "lines": 1
       }
      }
     ]
    },
    "b_in_a": {
     "lines": 122,
     "exact": 33,
     "fuzzy": 0,
     "in_order": 33,
     "containment": 0.27049180327868855,
     "exact_containment": 0.27049180327868855,
     "ordered_containment": 0.27049180327868855,
     "files": [
      {
       "path": "c/ringbuf.c",
       "lines": 20,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 26,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "lines": 18,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "lines": 14,
       "exact": 14,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 17,
       "exact": 17,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 2,
       "exact": 2,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_next",
       "kind": "function",
       "start": 7,
       "end": 10,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_init",
       "kind": "function",
       "start": 12,
       "end": 21,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_set_overwrite",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_push",
       "kind": "function",
       "start": 29,
       "end": 44,
       "lines": 6,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_pop",
       "kind": "function",
       "start": 46,
       "end": 55,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_peek",
       "kind": "function",
       "start": 58,
       "end": 65,
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_dropped",
       "kind": "function",
       "start": 73,
       "end": 76,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "item": "ringbuf_t",
       "kind": "struct",
       "start": 7,
       "end": 15,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_csv",
       "kind": "function",
       "start": 18,
       "end": 24,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_summary",
       "kind": "function",
       "start": 27,
       "end": 37,
       "lines": 11,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "parse_row",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "read_samples",
       "kind": "function",
       "start": 27,
       "end": 33,
       "lines": 4,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "moving_average",
       "kind": "function",
       "start": 26,
       "end": 34,
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "median_filter",
       "kind": "function",
       "start": 37,
       "end": 46,
       "lines": 10,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "POLY",
       "kind": "const",
       "start": 6,
       "end": 6,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "POLY",
        "start": 6,
        "end": 6,
        "lines": 1
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "crc16",
       "kind": "function",
       "start": 8,
       "end": 18,
       "lines": 7,
       "exact": 7,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "crc16",
        "start": 8,
        "end": 18,
        "lines": 7
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests",
       "kind": "module",
       "start": 20,
       "end": 28,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "tests",
        "start": 20,
        "end": 28,
        "lines": 3
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests::check_value",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "tests::check_value",
        "start": 24,
        "end": 27,
        "lines": 3
       }
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "CRC_LEN",
       "kind": "const",
       "start": 10,
       "end": 10,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "CRC_LEN",
        "start": 10,
        "end": 10,
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "FrameError",
       "kind": "enum",
       "start": 18,
       "end": 23,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "FrameError",
        "start": 18,
        "end": 23,
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "fmt::Display for FrameError::fmt",
       "kind": "function",
       "start": 26,
       "end": 36,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "fmt::Display for FrameError::fmt",
        "start": 26,
        "end": 36,
        "lines": 2
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "parse",
       "kind": "function",
       "start": 41,
       "end": 57,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "parse",
        "start": 41,
        "end": 57,
        "lines": 3
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "check_crc",
       "kind": "function",
       "start": 59,
       "end": 71,
       "lines": 9,
       "exact": 9,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "check_crc",
        "start": 59,
        "end": 71,
        "lines": 9
       }
      },
      {
       "path": "rust/src/lib.rs",
       "item": "crc",
       "kind": "module",
       "start": 4,
       "end": 4,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/lib.rs",
        "item": "crc",
        "start": 4,
        "end": 4,
        "lines": 1
       }
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/lib.rs",
        "lines": 1
       }
      }
     ]
    },
    "relation": "a-in-b"
   }
  },
  {
   "branch": "ringbuf-overwrite",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "782fe6c31f5ef468a4c60ceb79c94b8f62a8dd85",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "ringbuf-overwrite",
     "oid": "1d0754e68358e8f6611079e892977774d7ca465e",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 2,
     "files": 2,
     "lines": 25,
     "comment_lines": 5,
     "files_read": 2
    },
    "b": {
     "rev": "dev",
     "oid": "e82b73da827cd6247c98f1678508d909232a930f",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 5,
     "files": 9,
     "lines": 122,
     "comment_lines": 14,
     "files_read": 9
    },
    "a_in_b": {
     "lines": 25,
     "exact": 20,
     "fuzzy": 4,
     "in_order": 20,
     "containment": 0.96,
     "exact_containment": 0.8,
     "ordered_containment": 0.8,
     "files": [
      {
       "path": "c/ringbuf.c",
       "lines": 20,
       "exact": 16,
       "fuzzy": 3
      },
      {
       "path": "c/ringbuf.h",
       "lines": 5,
       "exact": 4,
       "fuzzy": 1
      }
     ],
     "items": [
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_next",
       "kind": "function",
       "start": 7,
       "end": 10,
       "lines": 2,
       "exact": 1,
       "fuzzy": 1,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_next",
        "start": 7,
        "end": 10,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_init",
       "kind": "function",
       "start": 12,
       "end": 21,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_init",
        "start": 12,
        "end": 21,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_set_overwrite",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_set_overwrite",
        "start": 24,
        "end": 27,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_push",
       "kind": "function",
       "start": 29,
       "end": 44,
       "lines": 6,
       "exact": 6,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_push",
        "start": 29,
        "end": 44,
        "lines": 6
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_pop",
       "kind": "function",
       "start": 46,
       "end": 55,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_pop",
        "start": 46,
        "end": 55,
        "lines": 1
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_peek",
       "kind": "function",
       "start": 58,
       "end": 65,
       "lines": 5,
       "exact": 2,
       "fuzzy": 2,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_peek",
        "start": 58,
        "end": 65,
        "lines": 4
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_dropped",
       "kind": "function",
       "start": 73,
       "end": 76,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_dropped",
        "start": 73,
        "end": 76,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.h",
       "item": "ringbuf_t",
       "kind": "struct",
       "start": 7,
       "end": 15,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.h",
        "item": "ringbuf_t",
        "start": 7,
        "end": 15,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.h",
       "lines": 3,
       "exact": 2,
       "fuzzy": 1,
       "counterpart": {
        "path": "c/ringbuf.h",
        "lines": 3
       }
      }
     ]
    },
    "b_in_a": {
     "lines": 122,
     "exact": 20,
     "fuzzy": 4,
     "in_order": 20,
     "containment": 0.19672131147540983,
     "exact_containment": 0.16393442622950818,
     "ordered_containment": 0.16393442622950818,
     "files": [
      {
       "path": "c/ringbuf.c",
       "lines": 20,
       "exact": 16,
       "fuzzy": 3
      },
      {
       "path": "c/ringbuf.h",
       "lines": 5,
       "exact": 4,
       "fuzzy": 1
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 26,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "lines": 18,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "lines": 14,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 17,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_next",
       "kind": "function",
       "start": 7,
       "end": 10,
       "lines": 2,
       "exact": 1,
       "fuzzy": 1,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_next",
        "start": 7,
        "end": 10,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_init",
       "kind": "function",
       "start": 12,
       "end": 21,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_init",
        "start": 12,
        "end": 21,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_set_overwrite",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_set_overwrite",
        "start": 24,
        "end": 27,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_push",
       "kind": "function",
       "start": 29,
       "end": 44,
       "lines": 6,
       "exact": 6,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_push",
        "start": 29,
        "end": 44,
        "lines": 6
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_pop",
       "kind": "function",
       "start": 46,
       "end": 55,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_pop",
        "start": 46,
        "end": 55,
        "lines": 1
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_peek",
       "kind": "function",
       "start": 58,
       "end": 65,
       "lines": 5,
       "exact": 2,
       "fuzzy": 2,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_peek",
        "start": 58,
        "end": 65,
        "lines": 4
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_dropped",
       "kind": "function",
       "start": 73,
       "end": 76,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_dropped",
        "start": 73,
        "end": 76,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.h",
       "item": "ringbuf_t",
       "kind": "struct",
       "start": 7,
       "end": 15,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.h",
        "item": "ringbuf_t",
        "start": 7,
        "end": 15,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.h",
       "lines": 3,
       "exact": 2,
       "fuzzy": 1,
       "counterpart": {
        "path": "c/ringbuf.h",
        "lines": 3
       }
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_csv",
       "kind": "function",
       "start": 18,
       "end": 24,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_summary",
       "kind": "function",
       "start": 27,
       "end": 37,
       "lines": 11,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "parse_row",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "read_samples",
       "kind": "function",
       "start": 27,
       "end": 33,
       "lines": 4,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "moving_average",
       "kind": "function",
       "start": 26,
       "end": 34,
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "median_filter",
       "kind": "function",
       "start": 37,
       "end": 46,
       "lines": 10,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "POLY",
       "kind": "const",
       "start": 6,
       "end": 6,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "crc16",
       "kind": "function",
       "start": 8,
       "end": 18,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests",
       "kind": "module",
       "start": 20,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests::check_value",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "CRC_LEN",
       "kind": "const",
       "start": 10,
       "end": 10,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "FrameError",
       "kind": "enum",
       "start": 18,
       "end": 23,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "fmt::Display for FrameError::fmt",
       "kind": "function",
       "start": 26,
       "end": 36,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "parse",
       "kind": "function",
       "start": 41,
       "end": 57,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "check_crc",
       "kind": "function",
       "start": 59,
       "end": 71,
       "lines": 9,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "item": "crc",
       "kind": "module",
       "start": 4,
       "end": 4,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "a-in-b"
   }
  },
  {
   "branch": "restructure-filters-and-crc",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "782fe6c31f5ef468a4c60ceb79c94b8f62a8dd85",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "restructure-filters-and-crc",
     "oid": "f22bbe5ff8363d484171d4a7ed637eb2af7f5783",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 2,
     "files": 5,
     "lines": 56,
     "comment_lines": 9,
     "files_read": 5
    },
    "b": {
     "rev": "dev",
     "oid": "e82b73da827cd6247c98f1678508d909232a930f",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 5,
     "files": 9,
     "lines": 122,
     "comment_lines": 14,
     "files_read": 9
    },
    "a_in_b": {
     "lines": 56,
     "exact": 47,
     "fuzzy": 0,
     "in_order": 47,
     "containment": 0.8392857142857143,
     "exact_containment": 0.8392857142857143,
     "ordered_containment": 0.8392857142857143,
     "files": [
      {
       "path": "python/telemetry/__init__.py",
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/smoothing.py",
       "lines": 21,
       "exact": 18,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "lines": 14,
       "exact": 14,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 18,
       "exact": 14,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "python/telemetry/__init__.py",
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/smoothing.py",
       "lines": 3,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/export.py",
        "lines": 1
       }
      },
      {
       "path": "python/telemetry/smoothing.py",
       "item": "rolling_mean",
       "kind": "function",
       "start": 11,
       "end": 19,
       "lines": 8,
       "exact": 7,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/stats.py",
        "item": "moving_average",
        "start": 26,
        "end": 34,
        "lines": 7
       }
      },
      {
       "path": "python/telemetry/smoothing.py",
       "item": "median_filter",
       "kind": "function",
       "start": 22,
       "end": 31,
       "lines": 10,
       "exact": 10,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/stats.py",
        "item": "median_filter",
        "start": 37,
        "end": 46,
        "lines": 10
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "POLY",
       "kind": "const",
       "start": 6,
       "end": 6,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "POLY",
        "start": 6,
        "end": 6,
        "lines": 1
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "crc16",
       "kind": "function",
       "start": 8,
       "end": 18,
       "lines": 7,
       "exact": 7,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "crc16",
        "start": 8,
        "end": 18,
        "lines": 7
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests",
       "kind": "module",
       "start": 20,
       "end": 28,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "tests",
        "start": 20,
        "end": 28,
        "lines": 3
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests::check_value",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "tests::check_value",
        "start": 24,
        "end": 27,
        "lines": 3
       }
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "CRC_LEN",
       "kind": "const",
       "start": 10,
       "end": 10,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "CRC_LEN",
        "start": 10,
        "end": 10,
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "FrameError",
       "kind": "enum",
       "start": 18,
       "end": 23,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "FrameError",
        "start": 18,
        "end": 23,
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "fmt::Display for FrameError::fmt",
       "kind": "function",
       "start": 26,
       "end": 36,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "fmt::Display for FrameError::fmt",
        "start": 26,
        "end": 36,
        "lines": 2
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "parse",
       "kind": "function",
       "start": 41,
       "end": 57,
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "parse",
        "start": 41,
        "end": 57,
        "lines": 2
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "Frame",
       "kind": "impl",
       "start": 59,
       "end": 73,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "Frame::verify_checksum",
       "kind": "function",
       "start": 60,
       "end": 72,
       "lines": 9,
       "exact": 7,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "check_crc",
        "start": 59,
        "end": 71,
        "lines": 7
       }
      },
      {
       "path": "rust/src/lib.rs",
       "item": "crc",
       "kind": "module",
       "start": 4,
       "end": 4,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/lib.rs",
        "item": "crc",
        "start": 4,
        "end": 4,
        "lines": 1
       }
      }
     ]
    },
    "b_in_a": {
     "lines": 122,
     "exact": 47,
     "fuzzy": 0,
     "in_order": 47,
     "containment": 0.38524590163934425,
     "exact_containment": 0.38524590163934425,
     "ordered_containment": 0.38524590163934425,
     "files": [
      {
       "path": "c/ringbuf.c",
       "lines": 20,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 26,
       "exact": 1,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "lines": 18,
       "exact": 17,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "lines": 14,
       "exact": 14,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 17,
       "exact": 14,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 2,
       "exact": 1,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_next",
       "kind": "function",
       "start": 7,
       "end": 10,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_init",
       "kind": "function",
       "start": 12,
       "end": 21,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_set_overwrite",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_push",
       "kind": "function",
       "start": 29,
       "end": 44,
       "lines": 6,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_pop",
       "kind": "function",
       "start": 46,
       "end": 55,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_peek",
       "kind": "function",
       "start": 58,
       "end": 65,
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_dropped",
       "kind": "function",
       "start": 73,
       "end": 76,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "item": "ringbuf_t",
       "kind": "struct",
       "start": 7,
       "end": 15,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 8,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/smoothing.py",
        "lines": 1
       }
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_csv",
       "kind": "function",
       "start": 18,
       "end": 24,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_summary",
       "kind": "function",
       "start": 27,
       "end": 37,
       "lines": 11,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "parse_row",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "read_samples",
       "kind": "function",
       "start": 27,
       "end": 33,
       "lines": 4,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "moving_average",
       "kind": "function",
       "start": 26,
       "end": 34,
       "lines": 8,
       "exact": 7,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/smoothing.py",
        "item": "rolling_mean",
        "start": 11,
        "end": 19,
        "lines": 7
       }
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "median_filter",
       "kind": "function",
       "start": 37,
       "end": 46,
       "lines": 10,
       "exact": 10,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/smoothing.py",
        "item": "median_filter",
        "start": 22,
        "end": 31,
        "lines": 10
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "POLY",
       "kind": "const",
       "start": 6,
       "end": 6,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "POLY",
        "start": 6,
        "end": 6,
        "lines": 1
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "crc16",
       "kind": "function",
       "start": 8,
       "end": 18,
       "lines": 7,
       "exact": 7,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "crc16",
        "start": 8,
        "end": 18,
        "lines": 7
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests",
       "kind": "module",
       "start": 20,
       "end": 28,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "tests",
        "start": 20,
        "end": 28,
        "lines": 3
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests::check_value",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "tests::check_value",
        "start": 24,
        "end": 27,
        "lines": 3
       }
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "CRC_LEN",
       "kind": "const",
       "start": 10,
       "end": 10,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "CRC_LEN",
        "start": 10,
        "end": 10,
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "FrameError",
       "kind": "enum",
       "start": 18,
       "end": 23,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "FrameError",
        "start": 18,
        "end": 23,
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "fmt::Display for FrameError::fmt",
       "kind": "function",
       "start": 26,
       "end": 36,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "fmt::Display for FrameError::fmt",
        "start": 26,
        "end": 36,
        "lines": 2
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "parse",
       "kind": "function",
       "start": 41,
       "end": 57,
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "parse",
        "start": 41,
        "end": 57,
        "lines": 2
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "check_crc",
       "kind": "function",
       "start": 59,
       "end": 71,
       "lines": 9,
       "exact": 7,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "Frame::verify_checksum",
        "start": 60,
        "end": 72,
        "lines": 7
       }
      },
      {
       "path": "rust/src/lib.rs",
       "item": "crc",
       "kind": "module",
       "start": 4,
       "end": 4,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/lib.rs",
        "item": "crc",
        "start": 4,
        "end": 4,
        "lines": 1
       }
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "a-in-b"
   }
  },
  {
   "branch": "docs-and-license-headers",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "782fe6c31f5ef468a4c60ceb79c94b8f62a8dd85",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "docs-and-license-headers",
     "oid": "bc9babe3142f289f69300ecfb7ae197fe2b9dc7b",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 2,
     "files": 1,
     "lines": 2,
     "comment_lines": 16,
     "files_read": 5
    },
    "b": {
     "rev": "dev",
     "oid": "e82b73da827cd6247c98f1678508d909232a930f",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 5,
     "files": 9,
     "lines": 122,
     "comment_lines": 14,
     "files_read": 9
    },
    "a_in_b": {
     "lines": 2,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "python/telemetry/__init__.py",
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "python/telemetry/__init__.py",
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 122,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "c/ringbuf.c",
       "lines": 20,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 26,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "lines": 18,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "lines": 14,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 17,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_next",
       "kind": "function",
       "start": 7,
       "end": 10,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_init",
       "kind": "function",
       "start": 12,
       "end": 21,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_set_overwrite",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_push",
       "kind": "function",
       "start": 29,
       "end": 44,
       "lines": 6,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_pop",
       "kind": "function",
       "start": 46,
       "end": 55,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_peek",
       "kind": "function",
       "start": 58,
       "end": 65,
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_dropped",
       "kind": "function",
       "start": 73,
       "end": 76,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "item": "ringbuf_t",
       "kind": "struct",
       "start": 7,
       "end": 15,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_csv",
       "kind": "function",
       "start": 18,
       "end": 24,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_summary",
       "kind": "function",
       "start": 27,
       "end": 37,
       "lines": 11,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "parse_row",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "read_samples",
       "kind": "function",
       "start": 27,
       "end": 33,
       "lines": 4,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "moving_average",
       "kind": "function",
       "start": 26,
       "end": 34,
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "median_filter",
       "kind": "function",
       "start": 37,
       "end": 46,
       "lines": 10,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "POLY",
       "kind": "const",
       "start": 6,
       "end": 6,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "crc16",
       "kind": "function",
       "start": 8,
       "end": 18,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests",
       "kind": "module",
       "start": 20,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests::check_value",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "CRC_LEN",
       "kind": "const",
       "start": 10,
       "end": 10,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "FrameError",
       "kind": "enum",
       "start": 18,
       "end": 23,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "fmt::Display for FrameError::fmt",
       "kind": "function",
       "start": 26,
       "end": 36,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "parse",
       "kind": "function",
       "start": 41,
       "end": 57,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "check_crc",
       "kind": "function",
       "start": 59,
       "end": 71,
       "lines": 9,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "item": "crc",
       "kind": "module",
       "start": 4,
       "end": 4,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "unrelated"
   }
  },
  {
   "branch": "dev-rebased",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "782fe6c31f5ef468a4c60ceb79c94b8f62a8dd85",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "dev-rebased",
     "oid": "a42b0691c40826a812d52630ea075dc425809d5a",
     "fork": "782fe6c31f5ef468a4c60ceb79c94b8f62a8dd85",
     "commits": 5,
     "files": 9,
     "lines": 122,
     "comment_lines": 14,
     "files_read": 9
    },
    "b": {
     "rev": "dev",
     "oid": "e82b73da827cd6247c98f1678508d909232a930f",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 5,
     "files": 9,
     "lines": 122,
     "comment_lines": 14,
     "files_read": 9
    },
    "a_in_b": {
     "lines": 122,
     "exact": 122,
     "fuzzy": 0,
     "in_order": 122,
     "containment": 1.0,
     "exact_containment": 1.0,
     "ordered_containment": 1.0,
     "files": [
      {
       "path": "c/ringbuf.c",
       "lines": 20,
       "exact": 20,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 5,
       "exact": 5,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 13,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 26,
       "exact": 26,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "lines": 7,
       "exact": 7,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "lines": 18,
       "exact": 18,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "lines": 14,
       "exact": 14,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 17,
       "exact": 17,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 2,
       "exact": 2,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_next",
       "kind": "function",
       "start": 7,
       "end": 10,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_next",
        "start": 7,
        "end": 10,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_init",
       "kind": "function",
       "start": 12,
       "end": 21,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_init",
        "start": 12,
        "end": 21,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_set_overwrite",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_set_overwrite",
        "start": 24,
        "end": 27,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_push",
       "kind": "function",
       "start": 29,
       "end": 44,
       "lines": 6,
       "exact": 6,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_push",
        "start": 29,
        "end": 44,
        "lines": 6
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_pop",
       "kind": "function",
       "start": 46,
       "end": 55,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_pop",
        "start": 46,
        "end": 55,
        "lines": 1
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_peek",
       "kind": "function",
       "start": 58,
       "end": 65,
       "lines": 5,
       "exact": 5,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_peek",
        "start": 58,
        "end": 65,
        "lines": 5
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_dropped",
       "kind": "function",
       "start": 73,
       "end": 76,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_dropped",
        "start": 73,
        "end": 76,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.h",
       "item": "ringbuf_t",
       "kind": "struct",
       "start": 7,
       "end": 15,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.h",
        "item": "ringbuf_t",
        "start": 7,
        "end": 15,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.h",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.h",
        "lines": 3
       }
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 13,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/__init__.py",
        "lines": 13
       }
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 8,
       "exact": 8,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/export.py",
        "lines": 8
       }
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_csv",
       "kind": "function",
       "start": 18,
       "end": 24,
       "lines": 7,
       "exact": 7,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/export.py",
        "item": "write_csv",
        "start": 18,
        "end": 24,
        "lines": 7
       }
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_summary",
       "kind": "function",
       "start": 27,
       "end": 37,
       "lines": 11,
       "exact": 11,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/export.py",
        "item": "write_summary",
        "start": 27,
        "end": 37,
        "lines": 11
       }
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "parse_row",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/reader.py",
        "item": "parse_row",
        "start": 22,
        "end": 24,
        "lines": 3
       }
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "read_samples",
       "kind": "function",
       "start": 27,
       "end": 33,
       "lines": 4,
       "exact": 4,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/reader.py",
        "item": "read_samples",
        "start": 27,
        "end": 33,
        "lines": 4
       }
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "moving_average",
       "kind": "function",
       "start": 26,
       "end": 34,
       "lines": 8,
       "exact": 8,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/stats.py",
        "item": "moving_average",
        "start": 26,
        "end": 34,
        "lines": 8
       }
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "median_filter",
       "kind": "function",
       "start": 37,
       "end": 46,
       "lines": 10,
       "exact": 10,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/stats.py",
        "item": "median_filter",
        "start": 37,
        "end": 46,
        "lines": 10
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "POLY",
       "kind": "const",
       "start": 6,
       "end": 6,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "POLY",
        "start": 6,
        "end": 6,
        "lines": 1
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "crc16",
       "kind": "function",
       "start": 8,
       "end": 18,
       "lines": 7,
       "exact": 7,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "crc16",
        "start": 8,
        "end": 18,
        "lines": 7
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests",
       "kind": "module",
       "start": 20,
       "end": 28,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "tests",
        "start": 20,
        "end": 28,
        "lines": 3
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests::check_value",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "tests::check_value",
        "start": 24,
        "end": 27,
        "lines": 3
       }
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "CRC_LEN",
       "kind": "const",
       "start": 10,
       "end": 10,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "CRC_LEN",
        "start": 10,
        "end": 10,
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "FrameError",
       "kind": "enum",
       "start": 18,
       "end": 23,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "FrameError",
        "start": 18,
        "end": 23,
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "fmt::Display for FrameError::fmt",
       "kind": "function",
       "start": 26,
       "end": 36,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "fmt::Display for FrameError::fmt",
        "start": 26,
        "end": 36,
        "lines": 2
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "parse",
       "kind": "function",
       "start": 41,
       "end": 57,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "parse",
        "start": 41,
        "end": 57,
        "lines": 3
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "check_crc",
       "kind": "function",
       "start": 59,
       "end": 71,
       "lines": 9,
       "exact": 9,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "check_crc",
        "start": 59,
        "end": 71,
        "lines": 9
       }
      },
      {
       "path": "rust/src/lib.rs",
       "item": "crc",
       "kind": "module",
       "start": 4,
       "end": 4,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/lib.rs",
        "item": "crc",
        "start": 4,
        "end": 4,
        "lines": 1
       }
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/lib.rs",
        "lines": 1
       }
      }
     ]
    },
    "b_in_a": {
     "lines": 122,
     "exact": 122,
     "fuzzy": 0,
     "in_order": 122,
     "containment": 1.0,
     "exact_containment": 1.0,
     "ordered_containment": 1.0,
     "files": [
      {
       "path": "c/ringbuf.c",
       "lines": 20,
       "exact": 20,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 5,
       "exact": 5,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 13,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 26,
       "exact": 26,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "lines": 7,
       "exact": 7,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "lines": 18,
       "exact": 18,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "lines": 14,
       "exact": 14,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 17,
       "exact": 17,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 2,
       "exact": 2,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_next",
       "kind": "function",
       "start": 7,
       "end": 10,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_next",
        "start": 7,
        "end": 10,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_init",
       "kind": "function",
       "start": 12,
       "end": 21,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_init",
        "start": 12,
        "end": 21,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_set_overwrite",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_set_overwrite",
        "start": 24,
        "end": 27,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_push",
       "kind": "function",
       "start": 29,
       "end": 44,
       "lines": 6,
       "exact": 6,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_push",
        "start": 29,
        "end": 44,
        "lines": 6
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_pop",
       "kind": "function",
       "start": 46,
       "end": 55,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_pop",
        "start": 46,
        "end": 55,
        "lines": 1
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_peek",
       "kind": "function",
       "start": 58,
       "end": 65,
       "lines": 5,
       "exact": 5,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_peek",
        "start": 58,
        "end": 65,
        "lines": 5
       }
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_dropped",
       "kind": "function",
       "start": 73,
       "end": 76,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.c",
        "item": "ringbuf_dropped",
        "start": 73,
        "end": 76,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.h",
       "item": "ringbuf_t",
       "kind": "struct",
       "start": 7,
       "end": 15,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.h",
        "item": "ringbuf_t",
        "start": 7,
        "end": 15,
        "lines": 2
       }
      },
      {
       "path": "c/ringbuf.h",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "c/ringbuf.h",
        "lines": 3
       }
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 13,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/__init__.py",
        "lines": 13
       }
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 8,
       "exact": 8,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/export.py",
        "lines": 8
       }
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_csv",
       "kind": "function",
       "start": 18,
       "end": 24,
       "lines": 7,
       "exact": 7,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/export.py",
        "item": "write_csv",
        "start": 18,
        "end": 24,
        "lines": 7
       }
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_summary",
       "kind": "function",
       "start": 27,
       "end": 37,
       "lines": 11,
       "exact": 11,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/export.py",
        "item": "write_summary",
        "start": 27,
        "end": 37,
        "lines": 11
       }
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "parse_row",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/reader.py",
        "item": "parse_row",
        "start": 22,
        "end": 24,
        "lines": 3
       }
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "read_samples",
       "kind": "function",
       "start": 27,
       "end": 33,
       "lines": 4,
       "exact": 4,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/reader.py",
        "item": "read_samples",
        "start": 27,
        "end": 33,
        "lines": 4
       }
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "moving_average",
       "kind": "function",
       "start": 26,
       "end": 34,
       "lines": 8,
       "exact": 8,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/stats.py",
        "item": "moving_average",
        "start": 26,
        "end": 34,
        "lines": 8
       }
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "median_filter",
       "kind": "function",
       "start": 37,
       "end": 46,
       "lines": 10,
       "exact": 10,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/stats.py",
        "item": "median_filter",
        "start": 37,
        "end": 46,
        "lines": 10
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "POLY",
       "kind": "const",
       "start": 6,
       "end": 6,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "POLY",
        "start": 6,
        "end": 6,
        "lines": 1
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "crc16",
       "kind": "function",
       "start": 8,
       "end": 18,
       "lines": 7,
       "exact": 7,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "crc16",
        "start": 8,
        "end": 18,
        "lines": 7
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests",
       "kind": "module",
       "start": 20,
       "end": 28,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "tests",
        "start": 20,
        "end": 28,
        "lines": 3
       }
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests::check_value",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/crc.rs",
        "item": "tests::check_value",
        "start": 24,
        "end": 27,
        "lines": 3
       }
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "CRC_LEN",
       "kind": "const",
       "start": 10,
       "end": 10,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "CRC_LEN",
        "start": 10,
        "end": 10,
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "FrameError",
       "kind": "enum",
       "start": 18,
       "end": 23,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "FrameError",
        "start": 18,
        "end": 23,
        "lines": 1
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "fmt::Display for FrameError::fmt",
       "kind": "function",
       "start": 26,
       "end": 36,
       "lines": 2,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "fmt::Display for FrameError::fmt",
        "start": 26,
        "end": 36,
        "lines": 2
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "parse",
       "kind": "function",
       "start": 41,
       "end": 57,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "parse",
        "start": 41,
        "end": 57,
        "lines": 3
       }
      },
      {
       "path": "rust/src/frame.rs",
       "item": "check_crc",
       "kind": "function",
       "start": 59,
       "end": 71,
       "lines": 9,
       "exact": 9,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/frame.rs",
        "item": "check_crc",
        "start": 59,
        "end": 71,
        "lines": 9
       }
      },
      {
       "path": "rust/src/lib.rs",
       "item": "crc",
       "kind": "module",
       "start": 4,
       "end": 4,
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/lib.rs",
        "item": "crc",
        "start": 4,
        "end": 4,
        "lines": 1
       }
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "rust/src/lib.rs",
        "lines": 1
       }
      }
     ]
    },
    "relation": "same"
   }
  },
  {
   "branch": "csv-export-and-plots",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "782fe6c31f5ef468a4c60ceb79c94b8f62a8dd85",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "csv-export-and-plots",
     "oid": "1b1cb7c25f573a4ac674abfa194d61a96bd93652",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 2,
     "files": 2,
     "lines": 29,
     "comment_lines": 4,
     "files_read": 2
    },
    "b": {
     "rev": "dev",
     "oid": "e82b73da827cd6247c98f1678508d909232a930f",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 5,
     "files": 9,
     "lines": 122,
     "comment_lines": 14,
     "files_read": 9
    },
    "a_in_b": {
     "lines": 29,
     "exact": 12,
     "fuzzy": 3,
     "in_order": 12,
     "containment": 0.5172413793103449,
     "exact_containment": 0.41379310344827586,
     "ordered_containment": 0.41379310344827586,
     "files": [
      {
       "path": "python/telemetry/export.py",
       "lines": 13,
       "exact": 12,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/plot.py",
       "lines": 16,
       "exact": 0,
       "fuzzy": 3
      }
     ],
     "items": [
      {
       "path": "python/telemetry/export.py",
       "lines": 6,
       "exact": 5,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/export.py",
        "lines": 5
       }
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_csv",
       "kind": "function",
       "start": 16,
       "end": 22,
       "lines": 7,
       "exact": 7,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/export.py",
        "item": "write_csv",
        "start": 18,
        "end": 24,
        "lines": 7
       }
      },
      {
       "path": "python/telemetry/plot.py",
       "lines": 5,
       "exact": 0,
       "fuzzy": 3,
       "counterpart": {
        "path": "python/telemetry/export.py",
        "lines": 3
       }
      },
      {
       "path": "python/telemetry/plot.py",
       "item": "plot_channels",
       "kind": "function",
       "start": 15,
       "end": 25,
       "lines": 11,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 122,
     "exact": 12,
     "fuzzy": 3,
     "in_order": 12,
     "containment": 0.12295081967213115,
     "exact_containment": 0.09836065573770492,
     "ordered_containment": 0.09836065573770492,
     "files": [
      {
       "path": "c/ringbuf.c",
       "lines": 20,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 26,
       "exact": 12,
       "fuzzy": 3
      },
      {
       "path": "python/telemetry/reader.py",
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "lines": 18,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "lines": 14,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 17,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_next",
       "kind": "function",
       "start": 7,
       "end": 10,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_init",
       "kind": "function",
       "start": 12,
       "end": 21,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_set_overwrite",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_push",
       "kind": "function",
       "start": 29,
       "end": 44,
       "lines": 6,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_pop",
       "kind": "function",
       "start": 46,
       "end": 55,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_peek",
       "kind": "function",
       "start": 58,
       "end": 65,
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_dropped",
       "kind": "function",
       "start": 73,
       "end": 76,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "item": "ringbuf_t",
       "kind": "struct",
       "start": 7,
       "end": 15,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 8,
       "exact": 5,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/export.py",
        "lines": 5
       }
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_csv",
       "kind": "function",
       "start": 18,
       "end": 24,
       "lines": 7,
       "exact": 7,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/export.py",
        "item": "write_csv",
        "start": 16,
        "end": 22,
        "lines": 7
       }
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_summary",
       "kind": "function",
       "start": 27,
       "end": 37,
       "lines": 11,
       "exact": 0,
       "fuzzy": 3,
       "counterpart": {
        "path": "python/telemetry/export.py",
        "item": "write_csv",
        "start": 16,
        "end": 22,
        "lines": 3
       }
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "parse_row",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "read_samples",
       "kind": "function",
       "start": 27,
       "end": 33,
       "lines": 4,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "moving_average",
       "kind": "function",
       "start": 26,
       "end": 34,
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "median_filter",
       "kind": "function",
       "start": 37,
       "end": 46,
       "lines": 10,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "POLY",
       "kind": "const",
       "start": 6,
       "end": 6,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "crc16",
       "kind": "function",
       "start": 8,
       "end": 18,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests",
       "kind": "module",
       "start": 20,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests::check_value",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "CRC_LEN",
       "kind": "const",
       "start": 10,
       "end": 10,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "FrameError",
       "kind": "enum",
       "start": 18,
       "end": 23,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "fmt::Display for FrameError::fmt",
       "kind": "function",
       "start": 26,
       "end": 36,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "parse",
       "kind": "function",
       "start": 41,
       "end": 57,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "check_crc",
       "kind": "function",
       "start": 59,
       "end": 71,
       "lines": 9,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "item": "crc",
       "kind": "module",
       "start": 4,
       "end": 4,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "overlap"
   }
  },
  {
   "branch": "serial-capture",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "782fe6c31f5ef468a4c60ceb79c94b8f62a8dd85",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "serial-capture",
     "oid": "b24a4a179828189895b68c3a35f2a6fa7ca4fc55",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 1,
     "files": 1,
     "lines": 24,
     "comment_lines": 2,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "e82b73da827cd6247c98f1678508d909232a930f",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 5,
     "files": 9,
     "lines": 122,
     "comment_lines": 14,
     "files_read": 9
    },
    "a_in_b": {
     "lines": 24,
     "exact": 2,
     "fuzzy": 0,
     "in_order": 2,
     "containment": 0.08333333333333333,
     "exact_containment": 0.08333333333333333,
     "ordered_containment": 0.08333333333333333,
     "files": [
      {
       "path": "python/telemetry/capture.py",
       "lines": 24,
       "exact": 2,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "python/telemetry/capture.py",
       "lines": 6,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/export.py",
        "lines": 2
       }
      },
      {
       "path": "python/telemetry/capture.py",
       "item": "capture",
       "kind": "function",
       "start": 15,
       "end": 32,
       "lines": 18,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 122,
     "exact": 2,
     "fuzzy": 0,
     "in_order": 2,
     "containment": 0.01639344262295082,
     "exact_containment": 0.01639344262295082,
     "ordered_containment": 0.01639344262295082,
     "files": [
      {
       "path": "c/ringbuf.c",
       "lines": 20,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 26,
       "exact": 2,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "lines": 18,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "lines": 14,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 17,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_next",
       "kind": "function",
       "start": 7,
       "end": 10,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_init",
       "kind": "function",
       "start": 12,
       "end": 21,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_set_overwrite",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_push",
       "kind": "function",
       "start": 29,
       "end": 44,
       "lines": 6,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_pop",
       "kind": "function",
       "start": 46,
       "end": 55,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_peek",
       "kind": "function",
       "start": 58,
       "end": 65,
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_dropped",
       "kind": "function",
       "start": 73,
       "end": 76,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "item": "ringbuf_t",
       "kind": "struct",
       "start": 7,
       "end": 15,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 8,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/capture.py",
        "lines": 2
       }
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_csv",
       "kind": "function",
       "start": 18,
       "end": 24,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_summary",
       "kind": "function",
       "start": 27,
       "end": 37,
       "lines": 11,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "parse_row",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "read_samples",
       "kind": "function",
       "start": 27,
       "end": 33,
       "lines": 4,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "moving_average",
       "kind": "function",
       "start": 26,
       "end": 34,
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "median_filter",
       "kind": "function",
       "start": 37,
       "end": 46,
       "lines": 10,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "POLY",
       "kind": "const",
       "start": 6,
       "end": 6,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "crc16",
       "kind": "function",
       "start": 8,
       "end": 18,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests",
       "kind": "module",
       "start": 20,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests::check_value",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "CRC_LEN",
       "kind": "const",
       "start": 10,
       "end": 10,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "FrameError",
       "kind": "enum",
       "start": 18,
       "end": 23,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "fmt::Display for FrameError::fmt",
       "kind": "function",
       "start": 26,
       "end": 36,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "parse",
       "kind": "function",
       "start": 41,
       "end": 57,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "check_crc",
       "kind": "function",
       "start": 59,
       "end": 71,
       "lines": 9,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "item": "crc",
       "kind": "module",
       "start": 4,
       "end": 4,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "unrelated"
   }
  },
  {
   "branch": "channel-config",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "782fe6c31f5ef468a4c60ceb79c94b8f62a8dd85",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "channel-config",
     "oid": "a55ef45da6e6ddbd1c193383c5e8252dabce0e4d",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 1,
     "files": 1,
     "lines": 18,
     "comment_lines": 2,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "e82b73da827cd6247c98f1678508d909232a930f",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 5,
     "files": 9,
     "lines": 122,
     "comment_lines": 14,
     "files_read": 9
    },
    "a_in_b": {
     "lines": 18,
     "exact": 2,
     "fuzzy": 0,
     "in_order": 2,
     "containment": 0.1111111111111111,
     "exact_containment": 0.1111111111111111,
     "ordered_containment": 0.1111111111111111,
     "files": [
      {
       "path": "python/telemetry/config.py",
       "lines": 18,
       "exact": 2,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "python/telemetry/config.py",
       "lines": 5,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/export.py",
        "lines": 2
       }
      },
      {
       "path": "python/telemetry/config.py",
       "item": "Channel",
       "kind": "class",
       "start": 13,
       "end": 19,
       "lines": 6,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/config.py",
       "item": "load_channels",
       "kind": "function",
       "start": 22,
       "end": 29,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 122,
     "exact": 2,
     "fuzzy": 0,
     "in_order": 2,
     "containment": 0.01639344262295082,
     "exact_containment": 0.01639344262295082,
     "ordered_containment": 0.01639344262295082,
     "files": [
      {
       "path": "c/ringbuf.c",
       "lines": 20,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 26,
       "exact": 2,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "lines": 18,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "lines": 14,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 17,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_next",
       "kind": "function",
       "start": 7,
       "end": 10,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_init",
       "kind": "function",
       "start": 12,
       "end": 21,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_set_overwrite",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_push",
       "kind": "function",
       "start": 29,
       "end": 44,
       "lines": 6,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_pop",
       "kind": "function",
       "start": 46,
       "end": 55,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_peek",
       "kind": "function",
       "start": 58,
       "end": 65,
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_dropped",
       "kind": "function",
       "start": 73,
       "end": 76,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "item": "ringbuf_t",
       "kind": "struct",
       "start": 7,
       "end": 15,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 8,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "python/telemetry/config.py",
        "lines": 2
       }
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_csv",
       "kind": "function",
       "start": 18,
       "end": 24,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_summary",
       "kind": "function",
       "start": 27,
       "end": 37,
       "lines": 11,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "parse_row",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "read_samples",
       "kind": "function",
       "start": 27,
       "end": 33,
       "lines": 4,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "moving_average",
       "kind": "function",
       "start": 26,
       "end": 34,
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "median_filter",
       "kind": "function",
       "start": 37,
       "end": 46,
       "lines": 10,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "POLY",
       "kind": "const",
       "start": 6,
       "end": 6,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "crc16",
       "kind": "function",
       "start": 8,
       "end": 18,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests",
       "kind": "module",
       "start": 20,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests::check_value",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "CRC_LEN",
       "kind": "const",
       "start": 10,
       "end": 10,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "FrameError",
       "kind": "enum",
       "start": 18,
       "end": 23,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "fmt::Display for FrameError::fmt",
       "kind": "function",
       "start": 26,
       "end": 36,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "parse",
       "kind": "function",
       "start": 41,
       "end": 57,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "check_crc",
       "kind": "function",
       "start": 59,
       "end": 71,
       "lines": 9,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "item": "crc",
       "kind": "module",
       "start": 4,
       "end": 4,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "unrelated"
   }
  },
  {
   "branch": "framedump-cli",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "782fe6c31f5ef468a4c60ceb79c94b8f62a8dd85",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "framedump-cli",
     "oid": "7a561d3b9d4e480737e7285ce7dc5f897b4fa9c1",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 1,
     "files": 1,
     "lines": 23,
     "comment_lines": 3,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "e82b73da827cd6247c98f1678508d909232a930f",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 5,
     "files": 9,
     "lines": 122,
     "comment_lines": 14,
     "files_read": 9
    },
    "a_in_b": {
     "lines": 23,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "rust/src/bin/framedump.rs",
       "lines": 23,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "rust/src/bin/framedump.rs",
       "lines": 4,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/bin/framedump.rs",
       "item": "main",
       "kind": "function",
       "start": 12,
       "end": 38,
       "lines": 19,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 122,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "c/ringbuf.c",
       "lines": 20,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 26,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "lines": 18,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "lines": 14,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 17,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_next",
       "kind": "function",
       "start": 7,
       "end": 10,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_init",
       "kind": "function",
       "start": 12,
       "end": 21,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_set_overwrite",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_push",
       "kind": "function",
       "start": 29,
       "end": 44,
       "lines": 6,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_pop",
       "kind": "function",
       "start": 46,
       "end": 55,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_peek",
       "kind": "function",
       "start": 58,
       "end": 65,
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_dropped",
       "kind": "function",
       "start": 73,
       "end": 76,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "item": "ringbuf_t",
       "kind": "struct",
       "start": 7,
       "end": 15,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_csv",
       "kind": "function",
       "start": 18,
       "end": 24,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_summary",
       "kind": "function",
       "start": 27,
       "end": 37,
       "lines": 11,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "parse_row",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "read_samples",
       "kind": "function",
       "start": 27,
       "end": 33,
       "lines": 4,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "moving_average",
       "kind": "function",
       "start": 26,
       "end": 34,
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "median_filter",
       "kind": "function",
       "start": 37,
       "end": 46,
       "lines": 10,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "POLY",
       "kind": "const",
       "start": 6,
       "end": 6,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "crc16",
       "kind": "function",
       "start": 8,
       "end": 18,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests",
       "kind": "module",
       "start": 20,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests::check_value",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "CRC_LEN",
       "kind": "const",
       "start": 10,
       "end": 10,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "FrameError",
       "kind": "enum",
       "start": 18,
       "end": 23,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "fmt::Display for FrameError::fmt",
       "kind": "function",
       "start": 26,
       "end": 36,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "parse",
       "kind": "function",
       "start": 41,
       "end": 57,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "check_crc",
       "kind": "function",
       "start": 59,
       "end": 71,
       "lines": 9,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "item": "crc",
       "kind": "module",
       "start": 4,
       "end": 4,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "unrelated"
   }
  },
  {
   "branch": "tick-clock",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "782fe6c31f5ef468a4c60ceb79c94b8f62a8dd85",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "tick-clock",
     "oid": "86c4daab0797cb0b677ba9ee91deee181e08fb2b",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 1,
     "files": 2,
     "lines": 15,
     "comment_lines": 5,
     "files_read": 2
    },
    "b": {
     "rev": "dev",
     "oid": "e82b73da827cd6247c98f1678508d909232a930f",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 5,
     "files": 9,
     "lines": 122,
     "comment_lines": 14,
     "files_read": 9
    },
    "a_in_b": {
     "lines": 15,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "c/clock.c",
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/clock.h",
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "c/clock.c",
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/clock.c",
       "item": "clock_init",
       "kind": "function",
       "start": 8,
       "end": 11,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/clock.c",
       "item": "clock_ticks_to_us",
       "kind": "function",
       "start": 13,
       "end": 16,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/clock.c",
       "item": "clock_elapsed",
       "kind": "function",
       "start": 19,
       "end": 22,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/clock.h",
       "lines": 6,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/clock.h",
       "item": "CLOCK_H",
       "kind": "macro",
       "start": 5,
       "end": 5,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 122,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "c/ringbuf.c",
       "lines": 20,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 26,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "lines": 18,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "lines": 14,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 17,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_next",
       "kind": "function",
       "start": 7,
       "end": 10,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_init",
       "kind": "function",
       "start": 12,
       "end": 21,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_set_overwrite",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_push",
       "kind": "function",
       "start": 29,
       "end": 44,
       "lines": 6,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_pop",
       "kind": "function",
       "start": 46,
       "end": 55,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_peek",
       "kind": "function",
       "start": 58,
       "end": 65,
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_dropped",
       "kind": "function",
       "start": 73,
       "end": 76,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "item": "ringbuf_t",
       "kind": "struct",
       "start": 7,
       "end": 15,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_csv",
       "kind": "function",
       "start": 18,
       "end": 24,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_summary",
       "kind": "function",
       "start": 27,
       "end": 37,
       "lines": 11,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "parse_row",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "read_samples",
       "kind": "function",
       "start": 27,
       "end": 33,
       "lines": 4,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "moving_average",
       "kind": "function",
       "start": 26,
       "end": 34,
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "median_filter",
       "kind": "function",
       "start": 37,
       "end": 46,
       "lines": 10,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "POLY",
       "kind": "const",
       "start": 6,
       "end": 6,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "crc16",
       "kind": "function",
       "start": 8,
       "end": 18,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests",
       "kind": "module",
       "start": 20,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests::check_value",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "CRC_LEN",
       "kind": "const",
       "start": 10,
       "end": 10,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "FrameError",
       "kind": "enum",
       "start": 18,
       "end": 23,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "fmt::Display for FrameError::fmt",
       "kind": "function",
       "start": 26,
       "end": 36,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "parse",
       "kind": "function",
       "start": 41,
       "end": 57,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "check_crc",
       "kind": "function",
       "start": 59,
       "end": 71,
       "lines": 9,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "item": "crc",
       "kind": "module",
       "start": 4,
       "end": 4,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "unrelated"
   }
  },
  {
   "branch": "fix-readme-typo",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "782fe6c31f5ef468a4c60ceb79c94b8f62a8dd85",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "fix-readme-typo",
     "oid": "4c9ef197641d4ab52e754b1ab52401d1530e3766",
     "fork": "4c9ef197641d4ab52e754b1ab52401d1530e3766",
     "commits": 0,
     "files": 0,
     "lines": 0,
     "comment_lines": 0,
     "files_read": 0
    },
    "b": {
     "rev": "dev",
     "oid": "e82b73da827cd6247c98f1678508d909232a930f",
     "fork": "d72e29fa3760f71fbf8c81053c958db69ad64e9a",
     "commits": 5,
     "files": 9,
     "lines": 122,
     "comment_lines": 14,
     "files_read": 9
    },
    "a_in_b": {
     "lines": 0,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "files": []
    },
    "b_in_a": {
     "lines": 122,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "c/ringbuf.c",
       "lines": 20,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 26,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "lines": 18,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "lines": 14,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 17,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_next",
       "kind": "function",
       "start": 7,
       "end": 10,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_init",
       "kind": "function",
       "start": 12,
       "end": 21,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_set_overwrite",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_push",
       "kind": "function",
       "start": 29,
       "end": 44,
       "lines": 6,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_pop",
       "kind": "function",
       "start": 46,
       "end": 55,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_peek",
       "kind": "function",
       "start": 58,
       "end": 65,
       "lines": 5,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.c",
       "item": "ringbuf_dropped",
       "kind": "function",
       "start": 73,
       "end": 76,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "item": "ringbuf_t",
       "kind": "struct",
       "start": 7,
       "end": 15,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "c/ringbuf.h",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/__init__.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_csv",
       "kind": "function",
       "start": 18,
       "end": 24,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/export.py",
       "item": "write_summary",
       "kind": "function",
       "start": 27,
       "end": 37,
       "lines": 11,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "parse_row",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/reader.py",
       "item": "read_samples",
       "kind": "function",
       "start": 27,
       "end": 33,
       "lines": 4,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "moving_average",
       "kind": "function",
       "start": 26,
       "end": 34,
       "lines": 8,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "python/telemetry/stats.py",
       "item": "median_filter",
       "kind": "function",
       "start": 37,
       "end": 46,
       "lines": 10,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "POLY",
       "kind": "const",
       "start": 6,
       "end": 6,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "crc16",
       "kind": "function",
       "start": 8,
       "end": 18,
       "lines": 7,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests",
       "kind": "module",
       "start": 20,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/crc.rs",
       "item": "tests::check_value",
       "kind": "function",
       "start": 24,
       "end": 27,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "CRC_LEN",
       "kind": "const",
       "start": 10,
       "end": 10,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "FrameError",
       "kind": "enum",
       "start": 18,
       "end": 23,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "fmt::Display for FrameError::fmt",
       "kind": "function",
       "start": 26,
       "end": 36,
       "lines": 2,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "parse",
       "kind": "function",
       "start": 41,
       "end": 57,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/frame.rs",
       "item": "check_crc",
       "kind": "function",
       "start": 59,
       "end": 71,
       "lines": 9,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "item": "crc",
       "kind": "module",
       "start": 4,
       "end": 4,
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "rust/src/lib.rs",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "empty"
   }
  }
 ]
};
