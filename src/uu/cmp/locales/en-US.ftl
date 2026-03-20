cmp-about = Compare two binary files for differences.

  With no FILE, or when FILE is -, read standard input.
cmp-usage = cmp [OPTION]... FILE1 [FILE2 [SKIP1 [SKIP2]]]

# Help messages
cmp-help-bytes-limit = compare at most LIMIT bytes
cmp-help-ignore-initial = SKIP - skip first SKIP bytes of both inputs
                          SKIP1:SKIP2 - set SKIP for each input individually
cmp-help-print-bytes = print differing bytes
cmp-help-quiet = quiet, same as --silent
cmp-help-silent = suppress all normal output
cmp-help-verbose = output byte numbers and differing byte values

# Error messages
cmp-error-missing-operands = missing operand after '{ $util_name }'
cmp-error-invalid-value = invalid value '{ $value }' for option --{ $option }
cmp-error-invalid-value-overflow = invalid value '{ $value }' (too large) for option --{ $option }
cmp-error-invalid-value-unit = invalid unit in '{ $value }' for option --{ $option }
cmp-error-incompatible-options = options --{ $opt1 } and --{ $opt2 } are incompatible
cmp-error-is-directory = '{ $name }' is a directory
cmp-error-not-yet-implemented = the option '--{ $option}' is not yet implemented
# unclear if the centralized one can be used
base-common-extra-operand = extra operand {$operand}
