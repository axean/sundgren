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
    "a": {
     "rev": "fitting-extract",
     "oid": "edc7c8fad6d46ca36a7d6ad5d02bbcb71c1e435f",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 1,
     "lines": 27,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 66,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 27,
     "exact": 27,
     "fuzzy": 0,
     "in_order": 27,
     "containment": 1.0,
     "exact_containment": 1.0,
     "ordered_containment": 1.0,
     "files": [
      {
       "path": "app/fitting.py",
       "lines": 27,
       "exact": 27,
       "fuzzy": 0
      }
     ],
     "comment_lines": 2,
     "code_containment": 1.0,
     "items": [
      {
       "path": "app/fitting.py",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 2
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_0",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_1",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_2",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_3",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_4",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_5",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_6",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_7",
        "lines": 3
       }
      }
     ]
    },
    "b_in_a": {
     "lines": 66,
     "exact": 27,
     "fuzzy": 4,
     "in_order": 27,
     "containment": 0.4696969696969697,
     "exact_containment": 0.4090909090909091,
     "ordered_containment": 0.4090909090909091,
     "files": [
      {
       "path": "app/export.py",
       "lines": 18,
       "exact": 2,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 27,
       "exact": 25,
       "fuzzy": 2
      },
      {
       "path": "app/plots.py",
       "lines": 21,
       "exact": 0,
       "fuzzy": 2
      }
     ],
     "comment_lines": 6,
     "code_containment": 0.4166666666666667,
     "items": [
      {
       "path": "app/export.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "lines": 2
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/fitting.py",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_0",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_1",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_2",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_3",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_4",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_5",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_6",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_7",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/fitting.py",
        "lines": 2
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
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
    "a": {
     "rev": "plots-reviewed",
     "oid": "3cbfb74123bbf26b519901e4e14a108a78cac660",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 1,
     "lines": 21,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 66,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 21,
     "exact": 9,
     "fuzzy": 12,
     "in_order": 9,
     "containment": 1.0,
     "exact_containment": 0.42857142857142855,
     "ordered_containment": 0.42857142857142855,
     "files": [
      {
       "path": "app/plots.py",
       "lines": 21,
       "exact": 9,
       "fuzzy": 12
      }
     ],
     "comment_lines": 2,
     "code_containment": 1.0,
     "items": [
      {
       "path": "app/plots.py",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 2
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_0",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_1",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_2",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_3",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_4",
        "lines": 2
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_5",
        "lines": 2
       }
      }
     ]
    },
    "b_in_a": {
     "lines": 66,
     "exact": 9,
     "fuzzy": 16,
     "in_order": 9,
     "containment": 0.3787878787878788,
     "exact_containment": 0.13636363636363635,
     "ordered_containment": 0.13636363636363635,
     "files": [
      {
       "path": "app/export.py",
       "lines": 18,
       "exact": 2,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 27,
       "exact": 0,
       "fuzzy": 2
      },
      {
       "path": "app/plots.py",
       "lines": 21,
       "exact": 7,
       "fuzzy": 14
      }
     ],
     "comment_lines": 6,
     "code_containment": 0.31666666666666665,
     "items": [
      {
       "path": "app/export.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "lines": 2
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "lines": 2
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_0",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_1",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_2",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_3",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_4",
        "lines": 2
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 1,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_5",
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
    "a": {
     "rev": "dev-rebased",
     "oid": "0e05bc4a2e8710464833a6eb9976facdbef605f2",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 3,
     "lines": 66,
     "files_read": 3
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 66,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 66,
     "exact": 66,
     "fuzzy": 0,
     "in_order": 66,
     "containment": 1.0,
     "exact_containment": 1.0,
     "ordered_containment": 1.0,
     "files": [
      {
       "path": "app/export.py",
       "lines": 18,
       "exact": 18,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 27,
       "exact": 27,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 21,
       "exact": 21,
       "fuzzy": 0
      }
     ],
     "comment_lines": 6,
     "code_containment": 1.0,
     "items": [
      {
       "path": "app/export.py",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_0",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_1",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_2",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_3",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_4",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_0",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_1",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_2",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_3",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_4",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_5",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_6",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_7",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_0",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_1",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_2",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_3",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_4",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_5",
        "lines": 3
       }
      }
     ]
    },
    "b_in_a": {
     "lines": 66,
     "exact": 66,
     "fuzzy": 0,
     "in_order": 66,
     "containment": 1.0,
     "exact_containment": 1.0,
     "ordered_containment": 1.0,
     "files": [
      {
       "path": "app/export.py",
       "lines": 18,
       "exact": 18,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 27,
       "exact": 27,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 21,
       "exact": 21,
       "fuzzy": 0
      }
     ],
     "comment_lines": 6,
     "code_containment": 1.0,
     "items": [
      {
       "path": "app/export.py",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_0",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_1",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_2",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_3",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_4",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_0",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_1",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_2",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_3",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_4",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_5",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_6",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/fitting.py",
        "item": "fitting_step_7",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_0",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_1",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_2",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_3",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_4",
        "lines": 3
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/plots.py",
        "item": "plots_step_5",
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
    "a": {
     "rev": "export-and-more",
     "oid": "7fc44ac61a0bc4d7dff30abdfa2a4a395eff6b1c",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 21,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 66,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 21,
     "exact": 11,
     "fuzzy": 0,
     "in_order": 11,
     "containment": 0.5238095238095238,
     "exact_containment": 0.5238095238095238,
     "ordered_containment": 0.5238095238095238,
     "files": [
      {
       "path": "app/export.py",
       "lines": 21,
       "exact": 11,
       "fuzzy": 0
      }
     ],
     "comment_lines": 4,
     "code_containment": 0.4117647058823529,
     "items": [
      {
       "path": "app/export.py",
       "lines": 6,
       "exact": 5,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_0",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_1",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "csv_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "csv_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "csv_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 66,
     "exact": 11,
     "fuzzy": 11,
     "in_order": 11,
     "containment": 0.3333333333333333,
     "exact_containment": 0.16666666666666666,
     "ordered_containment": 0.16666666666666666,
     "files": [
      {
       "path": "app/export.py",
       "lines": 18,
       "exact": 9,
       "fuzzy": 9
      },
      {
       "path": "app/fitting.py",
       "lines": 27,
       "exact": 2,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 21,
       "exact": 0,
       "fuzzy": 2
      }
     ],
     "comment_lines": 6,
     "code_containment": 0.26666666666666666,
     "items": [
      {
       "path": "app/export.py",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_0",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 3,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_1",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 3,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_0",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 3,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_0",
        "lines": 3
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 3,
       "counterpart": {
        "path": "app/export.py",
        "item": "export_step_0",
        "lines": 3
       }
      },
      {
       "path": "app/fitting.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 2
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/export.py",
        "lines": 2
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
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
    "a": {
     "rev": "logging",
     "oid": "789bcb477e2218429ded1d80e99f0f733117e294",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 18,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 66,
     "files_read": 3
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
       "path": "app/log.py",
       "lines": 18,
       "exact": 2,
       "fuzzy": 0
      }
     ],
     "comment_lines": 2,
     "code_containment": 0.0,
     "items": [
      {
       "path": "app/log.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 2
       }
      },
      {
       "path": "app/log.py",
       "item": "log_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/log.py",
       "item": "log_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/log.py",
       "item": "log_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/log.py",
       "item": "log_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/log.py",
       "item": "log_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 66,
     "exact": 2,
     "fuzzy": 4,
     "in_order": 2,
     "containment": 0.09090909090909091,
     "exact_containment": 0.030303030303030304,
     "ordered_containment": 0.030303030303030304,
     "files": [
      {
       "path": "app/export.py",
       "lines": 18,
       "exact": 2,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 27,
       "exact": 0,
       "fuzzy": 2
      },
      {
       "path": "app/plots.py",
       "lines": 21,
       "exact": 0,
       "fuzzy": 2
      }
     ],
     "comment_lines": 6,
     "code_containment": 0.0,
     "items": [
      {
       "path": "app/export.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/log.py",
        "lines": 2
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/log.py",
        "lines": 2
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/log.py",
        "lines": 2
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
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
    "a": {
     "rev": "metrics",
     "oid": "09db388117be3364663690f6bb53c2196045401f",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 12,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 66,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 12,
     "exact": 2,
     "fuzzy": 0,
     "in_order": 2,
     "containment": 0.16666666666666666,
     "exact_containment": 0.16666666666666666,
     "ordered_containment": 0.16666666666666666,
     "files": [
      {
       "path": "app/metrics.py",
       "lines": 12,
       "exact": 2,
       "fuzzy": 0
      }
     ],
     "comment_lines": 2,
     "code_containment": 0.0,
     "items": [
      {
       "path": "app/metrics.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 2
       }
      },
      {
       "path": "app/metrics.py",
       "item": "metrics_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/metrics.py",
       "item": "metrics_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/metrics.py",
       "item": "metrics_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 66,
     "exact": 2,
     "fuzzy": 4,
     "in_order": 2,
     "containment": 0.09090909090909091,
     "exact_containment": 0.030303030303030304,
     "ordered_containment": 0.030303030303030304,
     "files": [
      {
       "path": "app/export.py",
       "lines": 18,
       "exact": 2,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 27,
       "exact": 0,
       "fuzzy": 2
      },
      {
       "path": "app/plots.py",
       "lines": 21,
       "exact": 0,
       "fuzzy": 2
      }
     ],
     "comment_lines": 6,
     "code_containment": 0.0,
     "items": [
      {
       "path": "app/export.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/metrics.py",
        "lines": 2
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/metrics.py",
        "lines": 2
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/metrics.py",
        "lines": 2
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
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
    "a": {
     "rev": "config-loader",
     "oid": "71109afc403d761fe3017dce205bb341f35156ec",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 15,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 66,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 15,
     "exact": 2,
     "fuzzy": 0,
     "in_order": 2,
     "containment": 0.13333333333333333,
     "exact_containment": 0.13333333333333333,
     "ordered_containment": 0.13333333333333333,
     "files": [
      {
       "path": "app/config_loader.py",
       "lines": 15,
       "exact": 2,
       "fuzzy": 0
      }
     ],
     "comment_lines": 2,
     "code_containment": 0.0,
     "items": [
      {
       "path": "app/config_loader.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 2
       }
      },
      {
       "path": "app/config_loader.py",
       "item": "config_loader_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/config_loader.py",
       "item": "config_loader_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/config_loader.py",
       "item": "config_loader_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/config_loader.py",
       "item": "config_loader_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 66,
     "exact": 2,
     "fuzzy": 4,
     "in_order": 2,
     "containment": 0.09090909090909091,
     "exact_containment": 0.030303030303030304,
     "ordered_containment": 0.030303030303030304,
     "files": [
      {
       "path": "app/export.py",
       "lines": 18,
       "exact": 2,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 27,
       "exact": 0,
       "fuzzy": 2
      },
      {
       "path": "app/plots.py",
       "lines": 21,
       "exact": 0,
       "fuzzy": 2
      }
     ],
     "comment_lines": 6,
     "code_containment": 0.0,
     "items": [
      {
       "path": "app/export.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/config_loader.py",
        "lines": 2
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/config_loader.py",
        "lines": 2
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/config_loader.py",
        "lines": 2
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
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
    "a": {
     "rev": "cli-flags",
     "oid": "4fb0ab1787af912ee702dd59e7bfb4874ed1a730",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 18,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 66,
     "files_read": 3
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
       "path": "app/cli_flags.py",
       "lines": 18,
       "exact": 2,
       "fuzzy": 0
      }
     ],
     "comment_lines": 2,
     "code_containment": 0.0,
     "items": [
      {
       "path": "app/cli_flags.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 2
       }
      },
      {
       "path": "app/cli_flags.py",
       "item": "cli_flags_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cli_flags.py",
       "item": "cli_flags_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cli_flags.py",
       "item": "cli_flags_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cli_flags.py",
       "item": "cli_flags_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cli_flags.py",
       "item": "cli_flags_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 66,
     "exact": 2,
     "fuzzy": 4,
     "in_order": 2,
     "containment": 0.09090909090909091,
     "exact_containment": 0.030303030303030304,
     "ordered_containment": 0.030303030303030304,
     "files": [
      {
       "path": "app/export.py",
       "lines": 18,
       "exact": 2,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 27,
       "exact": 0,
       "fuzzy": 2
      },
      {
       "path": "app/plots.py",
       "lines": 21,
       "exact": 0,
       "fuzzy": 2
      }
     ],
     "comment_lines": 6,
     "code_containment": 0.0,
     "items": [
      {
       "path": "app/export.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/cli_flags.py",
        "lines": 2
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/cli_flags.py",
        "lines": 2
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/cli_flags.py",
        "lines": 2
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
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
    "a": {
     "rev": "cache",
     "oid": "5779fa44c9c4946fe43b3a8cc7f2114ac5121622",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 21,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 66,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 21,
     "exact": 2,
     "fuzzy": 0,
     "in_order": 2,
     "containment": 0.09523809523809523,
     "exact_containment": 0.09523809523809523,
     "ordered_containment": 0.09523809523809523,
     "files": [
      {
       "path": "app/cache.py",
       "lines": 21,
       "exact": 2,
       "fuzzy": 0
      }
     ],
     "comment_lines": 2,
     "code_containment": 0.0,
     "items": [
      {
       "path": "app/cache.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 2
       }
      },
      {
       "path": "app/cache.py",
       "item": "cache_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cache.py",
       "item": "cache_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cache.py",
       "item": "cache_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cache.py",
       "item": "cache_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cache.py",
       "item": "cache_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/cache.py",
       "item": "cache_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 66,
     "exact": 2,
     "fuzzy": 4,
     "in_order": 2,
     "containment": 0.09090909090909091,
     "exact_containment": 0.030303030303030304,
     "ordered_containment": 0.030303030303030304,
     "files": [
      {
       "path": "app/export.py",
       "lines": 18,
       "exact": 2,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 27,
       "exact": 0,
       "fuzzy": 2
      },
      {
       "path": "app/plots.py",
       "lines": 21,
       "exact": 0,
       "fuzzy": 2
      }
     ],
     "comment_lines": 6,
     "code_containment": 0.0,
     "items": [
      {
       "path": "app/export.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/cache.py",
        "lines": 2
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/cache.py",
        "lines": 2
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/cache.py",
        "lines": 2
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
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
    "a": {
     "rev": "retry-policy",
     "oid": "87160c3b1a491612c2ff54dfc98441215dde8657",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 12,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 66,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 12,
     "exact": 2,
     "fuzzy": 0,
     "in_order": 2,
     "containment": 0.16666666666666666,
     "exact_containment": 0.16666666666666666,
     "ordered_containment": 0.16666666666666666,
     "files": [
      {
       "path": "app/retry_policy.py",
       "lines": 12,
       "exact": 2,
       "fuzzy": 0
      }
     ],
     "comment_lines": 2,
     "code_containment": 0.0,
     "items": [
      {
       "path": "app/retry_policy.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 2
       }
      },
      {
       "path": "app/retry_policy.py",
       "item": "retry_policy_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/retry_policy.py",
       "item": "retry_policy_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/retry_policy.py",
       "item": "retry_policy_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 66,
     "exact": 2,
     "fuzzy": 4,
     "in_order": 2,
     "containment": 0.09090909090909091,
     "exact_containment": 0.030303030303030304,
     "ordered_containment": 0.030303030303030304,
     "files": [
      {
       "path": "app/export.py",
       "lines": 18,
       "exact": 2,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 27,
       "exact": 0,
       "fuzzy": 2
      },
      {
       "path": "app/plots.py",
       "lines": 21,
       "exact": 0,
       "fuzzy": 2
      }
     ],
     "comment_lines": 6,
     "code_containment": 0.0,
     "items": [
      {
       "path": "app/export.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/retry_policy.py",
        "lines": 2
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/retry_policy.py",
        "lines": 2
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/retry_policy.py",
        "lines": 2
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
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
    "a": {
     "rev": "units",
     "oid": "2aebf5f9c130863a4a54c92ff1e5509bce2eaaa3",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 1,
     "lines": 15,
     "files_read": 1
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 66,
     "files_read": 3
    },
    "a_in_b": {
     "lines": 15,
     "exact": 2,
     "fuzzy": 0,
     "in_order": 2,
     "containment": 0.13333333333333333,
     "exact_containment": 0.13333333333333333,
     "ordered_containment": 0.13333333333333333,
     "files": [
      {
       "path": "app/units.py",
       "lines": 15,
       "exact": 2,
       "fuzzy": 0
      }
     ],
     "comment_lines": 2,
     "code_containment": 0.0,
     "items": [
      {
       "path": "app/units.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/export.py",
        "lines": 2
       }
      },
      {
       "path": "app/units.py",
       "item": "units_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/units.py",
       "item": "units_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/units.py",
       "item": "units_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/units.py",
       "item": "units_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      }
     ]
    },
    "b_in_a": {
     "lines": 66,
     "exact": 2,
     "fuzzy": 4,
     "in_order": 2,
     "containment": 0.09090909090909091,
     "exact_containment": 0.030303030303030304,
     "ordered_containment": 0.030303030303030304,
     "files": [
      {
       "path": "app/export.py",
       "lines": 18,
       "exact": 2,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 27,
       "exact": 0,
       "fuzzy": 2
      },
      {
       "path": "app/plots.py",
       "lines": 21,
       "exact": 0,
       "fuzzy": 2
      }
     ],
     "comment_lines": 6,
     "code_containment": 0.0,
     "items": [
      {
       "path": "app/export.py",
       "lines": 3,
       "exact": 2,
       "fuzzy": 0,
       "counterpart": {
        "path": "app/units.py",
        "lines": 2
       }
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/units.py",
        "lines": 2
       }
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 2,
       "counterpart": {
        "path": "app/units.py",
        "lines": 2
       }
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
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
    "a": {
     "rev": "readme-typo",
     "oid": "93dffb5e41832ae535271fb419aa082eeace719c",
     "fork": "93dffb5e41832ae535271fb419aa082eeace719c",
     "files": 0,
     "lines": 0,
     "files_read": 0
    },
    "b": {
     "rev": "dev",
     "oid": "03bd84249144ad9019635fdac3281c1bf320cb90",
     "fork": "05e1329dfcba6026654107149ebd18f450251b9a",
     "files": 3,
     "lines": 66,
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
     "lines": 66,
     "exact": 0,
     "fuzzy": 0,
     "in_order": 0,
     "containment": 0.0,
     "exact_containment": 0.0,
     "ordered_containment": 0.0,
     "files": [
      {
       "path": "app/export.py",
       "lines": 18,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 27,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 21,
       "exact": 0,
       "fuzzy": 0
      }
     ],
     "comment_lines": 6,
     "code_containment": 0.0,
     "items": [
      {
       "path": "app/export.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/export.py",
       "item": "export_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_5",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_6",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/fitting.py",
       "item": "fitting_step_7",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_0",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_1",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_2",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_3",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_4",
       "kind": "function",
       "lines": 3,
       "exact": 0,
       "fuzzy": 0
      },
      {
       "path": "app/plots.py",
       "item": "plots_step_5",
       "kind": "function",
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
