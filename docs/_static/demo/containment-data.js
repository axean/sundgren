window.sundgrenDemoContainment =
{
 "target": "dev",
 "base": "main",
 "results": [
  {
   "branch": "fitting-extract",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "93dffb5e41832ae535271fb419aa082eeace719c",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "fitting-extract",
     "oid": "edc7c8fad6d46ca36a7d6ad5d02bbcb71c1e435f",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 1,
     "lines": 25,
     "comment_lines": 2,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 60,
     "comment_lines": 6,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 25,
     "exact": 25,
     "fuzzy": 0,
     "in_order": 25,
     "containment": 1.0,
     "exact_containment": 1.0,
     "ordered_containment": 1.0,
     "files": [
      {
       "path": "app/fitting.py",
       "lines": 25,
       "exact": 25,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/fitting.py",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "lines": 1
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_0",
        "start": 6,
        "end": 8,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_1",
        "start": 10,
        "end": 12,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_2",
        "start": 14,
        "end": 16,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_3",
        "start": 18,
        "end": 20,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_4",
        "start": 22,
        "end": 24,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_5",
        "start": 26,
        "end": 28,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "start": 30,
       "end": 32,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_6",
        "start": 30,
        "end": 32,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "start": 34,
       "end": 36,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_7",
        "start": 34,
        "end": 36,
        "lines": 3
       }
      }
     ]
    },
    "b_in_a": {
     "lines": 60,
     "exact": 25,
     "fuzzy": 0,
     "in_order": 25,
     "containment": 0.4166666666666667,
     "exact_containment": 0.4166666666666667,
     "ordered_containment": 0.4166666666666667,
     "files": [
      {
       "path": "app/export.py",
       "lines": 16,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 25,
       "exact": 25,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 19,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/export.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "lines": 1
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_0",
        "start": 6,
        "end": 8,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_1",
        "start": 10,
        "end": 12,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_2",
        "start": 14,
        "end": 16,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_3",
        "start": 18,
        "end": 20,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_4",
        "start": 22,
        "end": 24,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_5",
        "start": 26,
        "end": 28,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "start": 30,
       "end": 32,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_6",
        "start": 30,
        "end": 32,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "start": 34,
       "end": 36,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_7",
        "start": 34,
        "end": 36,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "a-in-b"
   }
  },
  {
   "branch": "plots-reviewed",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "93dffb5e41832ae535271fb419aa082eeace719c",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "plots-reviewed",
     "oid": "3cbfb74123bbf26b519901e4e14a108a78cac660",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 1,
     "lines": 19,
     "comment_lines": 2,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 60,
     "comment_lines": 6,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 19,
     "exact": 7,
     "fuzzy": 12,
     "in_order": 7,
     "containment": 1.0,
     "exact_containment": 0.3684210526315789,
     "ordered_containment": 0.3684210526315789,
     "files": [
      {
       "path": "app/plots.py",
       "lines": 19,
       "exact": 7,
       "fuzzy": 12
      }
     ],
     "items": [
      {
       "path": "app/plots.py",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "lines": 1
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_0",
        "start": 6,
        "end": 8,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_1",
        "start": 10,
        "end": 12,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_2",
        "start": 14,
        "end": 16,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_3",
        "start": 18,
        "end": 20,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_4",
        "start": 22,
        "end": 24,
        "lines": 2
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_5",
        "start": 26,
        "end": 28,
        "lines": 2
       }
      }
     ]
    },
    "b_in_a": {
     "lines": 60,
     "exact": 7,
     "fuzzy": 12,
     "in_order": 7,
     "containment": 0.31666666666666665,
     "exact_containment": 0.11666666666666667,
     "ordered_containment": 0.11666666666666667,
     "files": [
      {
       "path": "app/export.py",
       "lines": 16,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 25,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 19,
       "exact": 7,
       "fuzzy": 12
      }
     ],
     "items": [
      {
       "path": "app/export.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "start": 30,
       "end": 32,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "start": 34,
       "end": 36,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "lines": 1
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_0",
        "start": 6,
        "end": 8,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_1",
        "start": 10,
        "end": 12,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_2",
        "start": 14,
        "end": 16,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_3",
        "start": 18,
        "end": 20,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_4",
        "start": 22,
        "end": 24,
        "lines": 2
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_5",
        "start": 26,
        "end": 28,
        "lines": 2
       }
      }
     ]
    },
    "relation": "a-in-b"
   }
  },
  {
   "branch": "dev-rebased",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "93dffb5e41832ae535271fb419aa082eeace719c",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "dev-rebased",
     "oid": "0e05bc4a2e8710464833a6eb9976facdbef605f2",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 3,
     "lines": 60,
     "comment_lines": 6,
     "files_read": 3
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 60,
     "comment_lines": 6,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 60,
     "exact": 60,
     "fuzzy": 0,
     "in_order": 60,
     "containment": 1.0,
     "exact_containment": 1.0,
     "ordered_containment": 1.0,
     "files": [
      {
       "path": "app/export.py",
       "lines": 16,
       "exact": 16,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 25,
       "exact": 25,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 19,
       "exact": 19,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/export.py",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 1
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_0",
        "start": 6,
        "end": 8,
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_1",
        "start": 10,
        "end": 12,
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_2",
        "start": 14,
        "end": 16,
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_3",
        "start": 18,
        "end": 20,
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_4",
        "start": 22,
        "end": 24,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "lines": 1
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_0",
        "start": 6,
        "end": 8,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_1",
        "start": 10,
        "end": 12,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_2",
        "start": 14,
        "end": 16,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_3",
        "start": 18,
        "end": 20,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_4",
        "start": 22,
        "end": 24,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_5",
        "start": 26,
        "end": 28,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "start": 30,
       "end": 32,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_6",
        "start": 30,
        "end": 32,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "start": 34,
       "end": 36,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_7",
        "start": 34,
        "end": 36,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "lines": 1
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_0",
        "start": 6,
        "end": 8,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_1",
        "start": 10,
        "end": 12,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_2",
        "start": 14,
        "end": 16,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_3",
        "start": 18,
        "end": 20,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_4",
        "start": 22,
        "end": 24,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_5",
        "start": 26,
        "end": 28,
        "lines": 3
       }
      }
     ]
    },
    "b_in_a": {
     "lines": 60,
     "exact": 60,
     "fuzzy": 0,
     "in_order": 60,
     "containment": 1.0,
     "exact_containment": 1.0,
     "ordered_containment": 1.0,
     "files": [
      {
       "path": "app/export.py",
       "lines": 16,
       "exact": 16,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 25,
       "exact": 25,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 19,
       "exact": 19,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/export.py",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 1
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_0",
        "start": 6,
        "end": 8,
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_1",
        "start": 10,
        "end": 12,
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_2",
        "start": 14,
        "end": 16,
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_3",
        "start": 18,
        "end": 20,
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_4",
        "start": 22,
        "end": 24,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "lines": 1
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_0",
        "start": 6,
        "end": 8,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_1",
        "start": 10,
        "end": 12,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_2",
        "start": 14,
        "end": 16,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_3",
        "start": 18,
        "end": 20,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_4",
        "start": 22,
        "end": 24,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_5",
        "start": 26,
        "end": 28,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "start": 30,
       "end": 32,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_6",
        "start": 30,
        "end": 32,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "start": 34,
       "end": 36,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_7",
        "start": 34,
        "end": 36,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "lines": 1
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_0",
        "start": 6,
        "end": 8,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_1",
        "start": 10,
        "end": 12,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_2",
        "start": 14,
        "end": 16,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_3",
        "start": 18,
        "end": 20,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_4",
        "start": 22,
        "end": 24,
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_5",
        "start": 26,
        "end": 28,
        "lines": 3
       }
      }
     ]
    },
    "relation": "same"
   }
  },
  {
   "branch": "export-and-more",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "93dffb5e41832ae535271fb419aa082eeace719c",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "export-and-more",
     "oid": "7fc44ac61a0bc4d7dff30abdfa2a4a395eff6b1c",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 17,
     "comment_lines": 4,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 60,
     "comment_lines": 6,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 17,
     "exact": 7,
     "fuzzy": 0,
     "in_order": 7,
     "containment": 0.4117647058823529,
     "exact_containment": 0.4117647058823529,
     "ordered_containment": 0.4117647058823529,
     "files": [
      {
       "path": "app/export.py",
       "lines": 17,
       "exact": 7,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/export.py",
       "lines": 2,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 1
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_0",
        "start": 6,
        "end": 8,
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_1",
        "start": 10,
        "end": 12,
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "csv_step_0",
       "kind": "function",
       "start": 19,
       "end": 21,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "csv_step_1",
       "kind": "function",
       "start": 23,
       "end": 25,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "csv_step_2",
       "kind": "function",
       "start": 27,
       "end": 29,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 60,
     "exact": 7,
     "fuzzy": 9,
     "in_order": 7,
     "containment": 0.26666666666666666,
     "exact_containment": 0.11666666666666667,
     "ordered_containment": 0.11666666666666667,
     "files": [
      {
       "path": "app/export.py",
       "lines": 16,
       "exact": 7,
       "fuzzy": 9
      },
      {
       "path": "app/fitting.py",
       "lines": 25,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 19,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/export.py",
       "lines": 1,
       "exact": 1,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 1
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_0",
        "start": 6,
        "end": 8,
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_1",
        "start": 10,
        "end": 12,
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 3,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_0",
        "start": 6,
        "end": 8,
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 3,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_0",
        "start": 6,
        "end": 8,
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 3,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_0",
        "start": 6,
        "end": 8,
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "start": 30,
       "end": 32,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "start": 34,
       "end": 36,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "overlap"
   }
  },
  {
   "branch": "logging",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "93dffb5e41832ae535271fb419aa082eeace719c",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "logging",
     "oid": "789bcb477e2218429ded1d80e99f0f733117e294",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 16,
     "comment_lines": 2,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 60,
     "comment_lines": 6,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 16,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/log.py",
       "lines": 16,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/log.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/log.py",
       "item": "log_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/log.py",
       "item": "log_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/log.py",
       "item": "log_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/log.py",
       "item": "log_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/log.py",
       "item": "log_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 60,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/export.py",
       "lines": 16,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 25,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 19,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/export.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "start": 30,
       "end": 32,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "start": 34,
       "end": 36,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "unrelated"
   }
  },
  {
   "branch": "metrics",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "93dffb5e41832ae535271fb419aa082eeace719c",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "metrics",
     "oid": "09db388117be3364663690f6bb53c2196045401f",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 10,
     "comment_lines": 2,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 60,
     "comment_lines": 6,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 10,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/metrics.py",
       "lines": 10,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/metrics.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/metrics.py",
       "item": "metrics_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/metrics.py",
       "item": "metrics_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/metrics.py",
       "item": "metrics_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 60,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/export.py",
       "lines": 16,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 25,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 19,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/export.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "start": 30,
       "end": 32,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "start": 34,
       "end": 36,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "unrelated"
   }
  },
  {
   "branch": "config-loader",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "93dffb5e41832ae535271fb419aa082eeace719c",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "config-loader",
     "oid": "71109afc403d761fe3017dce205bb341f35156ec",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 13,
     "comment_lines": 2,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 60,
     "comment_lines": 6,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 13,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/config_loader.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/config_loader.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/config_loader.py",
       "item": "config_loader_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/config_loader.py",
       "item": "config_loader_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/config_loader.py",
       "item": "config_loader_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/config_loader.py",
       "item": "config_loader_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 60,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/export.py",
       "lines": 16,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 25,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 19,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/export.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "start": 30,
       "end": 32,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "start": 34,
       "end": 36,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "unrelated"
   }
  },
  {
   "branch": "cli-flags",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "93dffb5e41832ae535271fb419aa082eeace719c",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "cli-flags",
     "oid": "4fb0ab1787af912ee702dd59e7bfb4874ed1a730",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 16,
     "comment_lines": 2,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 60,
     "comment_lines": 6,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 16,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/cli_flags.py",
       "lines": 16,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/cli_flags.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cli_flags.py",
       "item": "cli_flags_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cli_flags.py",
       "item": "cli_flags_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cli_flags.py",
       "item": "cli_flags_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cli_flags.py",
       "item": "cli_flags_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cli_flags.py",
       "item": "cli_flags_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 60,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/export.py",
       "lines": 16,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 25,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 19,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/export.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "start": 30,
       "end": 32,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "start": 34,
       "end": 36,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "unrelated"
   }
  },
  {
   "branch": "cache",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "93dffb5e41832ae535271fb419aa082eeace719c",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "cache",
     "oid": "5779fa44c9c4946fe43b3a8cc7f2114ac5121622",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 19,
     "comment_lines": 2,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 60,
     "comment_lines": 6,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 19,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/cache.py",
       "lines": 19,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/cache.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cache.py",
       "item": "cache_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cache.py",
       "item": "cache_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cache.py",
       "item": "cache_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cache.py",
       "item": "cache_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cache.py",
       "item": "cache_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cache.py",
       "item": "cache_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 60,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/export.py",
       "lines": 16,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 25,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 19,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/export.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "start": 30,
       "end": 32,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "start": 34,
       "end": 36,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "unrelated"
   }
  },
  {
   "branch": "retry-policy",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "93dffb5e41832ae535271fb419aa082eeace719c",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "retry-policy",
     "oid": "87160c3b1a491612c2ff54dfc98441215dde8657",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 10,
     "comment_lines": 2,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 60,
     "comment_lines": 6,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 10,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/retry_policy.py",
       "lines": 10,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/retry_policy.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/retry_policy.py",
       "item": "retry_policy_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/retry_policy.py",
       "item": "retry_policy_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/retry_policy.py",
       "item": "retry_policy_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 60,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/export.py",
       "lines": 16,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 25,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 19,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/export.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "start": 30,
       "end": 32,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "start": 34,
       "end": 36,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "unrelated"
   }
  },
  {
   "branch": "units",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "93dffb5e41832ae535271fb419aa082eeace719c",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "units",
     "oid": "2aebf5f9c130863a4a54c92ff1e5509bce2eaaa3",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 13,
     "comment_lines": 2,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 60,
     "comment_lines": 6,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 13,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/units.py",
       "lines": 13,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/units.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/units.py",
       "item": "units_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/units.py",
       "item": "units_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/units.py",
       "item": "units_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/units.py",
       "item": "units_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 60,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/export.py",
       "lines": 16,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 25,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 19,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/export.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "start": 30,
       "end": 32,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "start": 34,
       "end": 36,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "relation": "unrelated"
   }
  },
  {
   "branch": "readme-typo",
   "result": {
    "schema": "sundgren.containment/0",
    "repository": "/demo",
    "base": "main",
    "base_oid": "93dffb5e41832ae535271fb419aa082eeace719c",
    "threshold": 0.9,
    "comments": "ignored",
    "a": {
     "rev": "readme-typo",
     "oid": "93dffb5e41832ae535271fb419aa082eeace719c",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 0,
     "lines": 0,
     "comment_lines": 0,
     "files_read": 0
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 60,
     "comment_lines": 6,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 0,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "files": []
    },
    "b_in_a": {
     "lines": 60,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/export.py",
       "lines": 16,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 25,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 19,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "items": [
      {
       "path": "app/export.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "start": 30,
       "end": 32,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "start": 34,
       "end": 36,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 1,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "start": 6,
       "end": 8,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "start": 10,
       "end": 12,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "start": 14,
       "end": 16,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "start": 18,
       "end": 20,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "start": 22,
       "end": 24,
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "start": 26,
       "end": 28,
       "lines": 3,
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
