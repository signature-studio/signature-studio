#!/usr/bin/env python3
"""pick1: Learn page (saved HTML) -> plain text of the article, for grep. usage: html2txt.py FILE.html > FILE.txt"""
import re, sys, html
s = open(sys.argv[1], encoding='utf-8', errors='replace').read()
m = re.search(r'<main[^>]*>(.*?)</main>', s, re.S)
s = m.group(1) if m else s
s = re.sub(r'<(script|style)[^>]*>.*?</\1>', '', s, flags=re.S)
s = re.sub(r'<(br|/p|/tr|/li|/h\d|/div|/pre|/table)[^>]*>', '\n', s)
s = re.sub(r'<[^>]+>', '', s)
s = html.unescape(s)
s = re.sub(r'[ \t]+', ' ', s)
s = re.sub(r'\n\s*\n+', '\n', s)
print(s.strip())
