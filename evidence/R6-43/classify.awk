# Classify a fle_stop/STOP line into the limit kinds of decision 0424's scan.
# Order matters: the first matching class wins. Only lines that match at least one limit word are kept.
{
  line = tolower($0)
  if (line !~ /(limit|max|more than|longer than|larger than|too |exceed|exhaust|region|table|holds|live|alive|slots|full|at once|outstanding|overflow|deep|levels|frames|nested|components|mib|gib|bytes|characters|units|buffer|count|room|keeps|kept|tracked|capacity|depth|size|above 2|1 mb|past|reach|would pass|would reach)/) next
  if (line ~ /(0x80|0x7f|code page|non-ascii|utf-8|utf-16|ansi|printable|byte of|character of 0x80|unit of 0x80|0x7f|above 0x7)/) cls = "E"
  else if (line ~ /(overflow|underflow|uint32_max|reference count|refcount|initialization count|recursion overflow)/) cls = "C"
  else if (line ~ /(deep|levels|frames|nested|components|depth|hook procedures)/) cls = "D"
  else if (line ~ /(longer than|larger than|mib|gib|bytes|characters|units|max_path|buffer|too long|size|1 mb|2\^63|cj|bitmap is 2|dwnumberofbytes|max_size|maximum of)/) cls = "Z"
  else if (line ~ /(limit|region|table|holds|live|alive|slots|full|at once|outstanding|keeps|kept|tracked|capacity|room|too many|more than|exhaust|handle hooks|the list|the model|this runtime|this environment|this model|this table)/) cls = "T"
  else cls = "O"
  printf "%s\t%s\n", cls, $0
}
