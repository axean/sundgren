sundgren run --patch big.patch --tests 'tests/test_calc.py::test_classify[5-pos]' --format json --full -o report.json
sundgren run --patch big.patch --tests 'tests/test_calc.py::test_classify[5-pos]' --format text -o report.txt
