#!/usr/bin/python
# -*- coding: utf-8 -*-
# Copyright: Damien Elmes <anki@ichi2.net>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# Automatic reading generation with kakasi and mecab.
# See http://ichi2.net/anki/wiki/JapaneseSupport
#
# Adapted for stand-alone use by
# Christoph Dittmann <github@christoph-d.de>.

import sys, os, platform, re, subprocess, codecs

MAX_OUTPUT_LENGTH = 300

kakasiCmd = ["kakasi", "-iutf8", "-outf8", "-u", "-JH", "-KH"]
mecabCmd = ["mecab", '--node-format=%m[%f[5]] ', '--eos-format=\n',
            '--unk-format=%m[] ']

class KakasiController(object):
    def __init__(self):
        self.kakasi = None

    def ensureOpen(self):
        if not self.kakasi:
            try:
                self.kakasi = subprocess.Popen(
                    kakasiCmd, bufsize=-1, stdin=subprocess.PIPE,
                    stdout=subprocess.PIPE)
            except OSError:
                raise Exception("Please install kakasi.")

    def toHiragana(self, expr):
        self.ensureOpen()
        self.kakasi.stdin.write(expr.encode("utf8", "ignore")+'\n')
        self.kakasi.stdin.flush()
        res = unicode(self.kakasi.stdout.readline().rstrip('\r\n'), "utf8")
        return res

kakasi = KakasiController()

def fixExpr(expr):
    out = []
    expr_split = re.split("([^\[]+\[[^\]]*\])", expr)
    for node in expr_split:
        if node == '':
            continue
        m = re.match("(.+)\[(.*)\]", node.decode("utf-8"))
        if not m:
            out.append(node.decode("utf-8"))
            continue
        (kanji, reading) = m.groups()
        # hiragana, katakana, punctuation, not japanese, or lacking a reading
        if kanji == reading or not reading:
            out.append(kanji)
            continue
        # convert to hiragana
        reading = kakasi.toHiragana(reading)
        # ended up the same
        if reading == kanji:
            out.append(kanji)
            continue
        # don't add readings of numbers
        if kanji.strip() in u"０１２３４５６７８９": # u"一二三四五六七八九十０１２３４５６７８９":
            out.append(kanji)
            continue
        # strip matching characters and beginning and end of reading and kanji
        # reading should always be at least as long as the kanji
        placeL = 0
        placeR = 0
        for i in range(1,len(kanji)):
            if kanji[-i] != reading[-i]:
                break
            placeR = i
        for i in range(0,len(kanji)-1):
            if kanji[i] != reading[i]:
                break
            placeL = i+1
        if placeL == 0:
            if placeR == 0:
                out.append(" %s[%s]" % (kanji, reading))
            else:
                out.append(" %s[%s]%s" % (
                    kanji[:-placeR], reading[:-placeR], reading[-placeR:]))
        else:
            if placeR == 0:
                out.append("%s %s[%s]" % (
                    reading[:placeL], kanji[placeL:], reading[placeL:]))
            else:
                out.append("%s %s[%s]%s" % (
                    reading[:placeL], kanji[placeL:-placeR],
                    reading[placeL:-placeR], reading[-placeR:]))
    fin = ""
    for c, s in enumerate(out):
        if c < len(out) - 1 and re.match("^[A-Za-z0-9]+$", out[c+1]):
            s += " "
        fin += s
    fin = fin.strip()
    fin = re.sub(u"\[\]", u"", fin)
    fin = re.sub(u" +", u" ", fin)
    return fin

def get_readings(expr):
    try:
        mecab = subprocess.Popen(
            mecabCmd, bufsize=-1, stdin=subprocess.PIPE,
            stdout=subprocess.PIPE)
        return mecab.communicate(expr)[0]
    except OSError:
        raise Exception("Please install mecab.")

if __name__ == "__main__":
    sys.stdout = codecs.open("/dev/stdout", "w", 'utf-8')
    if len(sys.argv) != 2 or len(sys.argv[1]) == 0:
        print 'Please provide one argument.'
        sys.exit(0)
    try:
        result = fixExpr(get_readings(sys.argv[1]))
        result = re.sub(u"\\n", u"", result)
    except Exception, (e):
        print e
        sys.exit(1)
    if len(result) > MAX_OUTPUT_LENGTH:
        print result[0:MAX_OUTPUT_LENGTH - 3] + u'...'
    else:
        print result
