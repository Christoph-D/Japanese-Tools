#!/usr/bin/python2
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# This script asks Google dictionary for English words.
#
# Thanks goes to klaxa ( https://github.com/klaxa ) for the first
# version of this script using the dictionary API.

import json, urllib, sys, re

MAX_OUTPUT_LENGTH = 300
API_URL="http://www.google.com/dictionary/json?callback=a&sl=en&tl=en&q="

if len(sys.argv) >= 2:
    word = sys.argv[1]
else:
    word = "empty"

fp = urllib.urlopen(API_URL + word)
stripped_string = urllib.unquote(re.sub("^a\((.*),[^,]+,[^,]+\)$", "\\1", fp.read()))
answer_string = stripped_string.decode('string-escape')
answer = json.loads(answer_string)

def find_phonetic(answer):
    for t in answer["primaries"][0]["terms"]:
        if t["type"] == u"phonetic":
            return t["text"] + " "
    return ""

def find_good_term(entries):
    for e in entries:
        if e["terms"][0]["language"] == u"en":
            return e["terms"][0]["text"]
    return ""

def construct_answer(answer):
    try:
        entries = " / ".join([ find_good_term(p["entries"]) for p in answer["primaries"] ])
        result = find_phonetic(answer) + entries
        # Remove HTML tags like <em>this</em>
        result = re.sub("</?[^>]*>", "", result)
        if len(result) > MAX_OUTPUT_LENGTH:
            result = result[0:MAX_OUTPUT_LENGTH-3] + "..."
        return result
    except:
        return "No result. :-("

print construct_answer(answer).encode("utf-8", errors="ignore")
