# Japanese tools / IRC bot

These are some scripts that help me learn Japanese.

Most scripts are supposed to be used as plugins for an IRC bot or run on a
shell. I find the following aliases quite useful:

```bash
alias ja="$JAPANESE_TOOLS/jmdict/jm.sh"
alias wa="$JAPANESE_TOOLS/jmdict/wa.sh"
alias rtk="$JAPANESE_TOOLS/rtk/rtk.sh"
```

I do most of my dictionary lookups with these aliases.

All scripts have only been tested on Ubuntu 24.04 and later. A few dependencies
are not present on a default Ubuntu system. You can install them with

```bash
sudo apt install mecab-jumandic-utf8 mecab kakasi xmlstarlet xsltproc python-irclib sqlite3 bc liburi-perl tesseract-ocr imagemagick
cargo install xtr  # development dependency
```

## ai/

Query an LLM. To set it up, go into the `ai` directory and run the following
commands:

```bash
cargo build --release && cp target/release/ai .
cp api-keys.example api-keys
# Edit api-keys and add your keys
```

Examples:

```text
$ ./ai
Usage: !ai [-model] <query>. Known models: deepseek-chat google/gemini-2.5-flash openai/gpt-4o-mini openai/gpt-4.1. Default: deepseek-chat
$ ./ai "What's 夜空?"
"夜空" means "night sky" in English.
$ ./ai "-google/gemini-2.5-flash What's 夜空? Also give me a Japanese example sentence."
[g] 夜空 (yozora) means "night sky," as in "夜空が綺麗ですね" (Yozora ga kirei desu ne), which means "The night sky is beautiful, isn't it?"
```

The following example demonstrates the AI's short-term memory.

```text
<Christoph>  !ai What is 夜空?
<nihongobot> 夜空 means "night sky" in Japanese, referring to the sky as seen after sunset.
<Christoph>  !ai Do you know any other words like this?
<nihongobot> Yes! 星空 (hoshi-zora) means "starry sky" and 夕空 (yuu-zora) means "evening sky."
<Christoph>  !ai Something for morning?
<nihongobot> 朝空 (asa-zora) means "morning sky" in Japanese.
```

## audio/

`find_audio.sh` finds an audio version of a given Japanese word on
languagepod101.

```text
$ ./find_audio.sh 夜空
Audio for 夜空 [よぞら]: http://tinyurl.com/p8aq8jo
```

## compare_encoding

Compares the size of different encodings of the same Japanese Wikipedia article.
In almost all cases UTF-8 is smaller than UTF-16.

```text
$ ./compare_encoding.sh 夜空
UTF-8 vs. UTF-16: 91213 vs. 156876 bytes. UTF-8 wins by 41.8%.
```

## gettext/

Internationalization support. Currently supported languages:

- English
- German
- Polish

Be sure to run `gettext/regenerate_mo_files.sh` if you would like to use a
translation.

## jmdict/

`jm.sh` provides jmdict lookups and `wa.sh` wadoku lookups. Works best for
Japanese->English (or Japanese->German), not so well for the reverse direction.
This is because jmdict is a Japanese English dictionary and not an English
Japanese dictionary.

To start, you first need to run the scripts `prepare_jmdict.sh` and
`prepare_wadoku.sh`. This will download and process the respective dictionary
files.

```text
$ ./jm.sh 村長
村長 [そんちょう] (n), village headman
市長村長選挙 [しちょうそんちょうせんきょ] (n), mayoral election
```

## kana/

A simple hiragana and katakana trainer.

### Example IRC session

```text
<Christoph>  !hira help
<nihongobot> Start with "!hira <level> [count]". Known levels are 0
             to 10. To learn more about some level please use
             "!hira help <level>".
<nihongobot> To only see the differences between consecutive
             levels, please use "!hira helpdiff <level>".
<Christoph>  !hira 5
<nihongobot> Please write in romaji: す と に ね へ
<Christoph>  !hira su to ni ne he
<nihongobot> Perfect! 5 of 5. Statistics for Christoph: 44.64% of
             280 characters correct.
<nihongobot> Please write in romaji: は と ぬ ほ な
```

## kanjidic/

Implements a lookup in kanjidic:
<http://www.csse.monash.edu.au/~jwb/kanjidic.html>

```text
$ ./kanjidic.sh 日本語
日: 4 strokes. ニチ, ジツ, ひ, -び, -か. In names: あ, あき, いる, く, くさ, こう, す, たち, に, にっ, につ, へ {day, sun, Japan, counter for days}
本: 5 strokes. ホン, もと. In names: まと {book, present, main, origin, true, real, counter for long cylindrical things}
語: 14 strokes. ゴ, かた.る, かた.らう {word, speech, language}
```

## kumitate_quiz/

A quiz asking JLPT style 文の組み立て questions. Only works as an IRC plugin for
now.

### Example IRC session

```text
<Flamerokz> !kuiz skm2
<nihongobot> Please choose [1-4]: 周囲の人たちの　＿　＿　★　＿　と思う。 (1: 協力を 2: 優勝は 3: 無理だった 4: 抜きにしては).
<Flamerokz> !kuiz 2
<nihongobot> Flamerokz: Correct! (2: 優勝は)
```

### Example question file

A question file (a file ending in `.txt` in `kumitate_quiz/questions/`) should
contains lines of the following form:

```text
周囲の人たちの　＿　＿　★　＿　と思う。|協力を,優勝は,無理だった,抜きにしては|2
```

## lhc

This script has nothing to do with Japanese. It OCRs the image on
<http://op-webtools.web.cern.ch/op-webtools/vistar/vistars.php?usr=LHC1> to
provide live statistics of the status of the LHC.

## reading/

`read.py` converts kanji to kana using mecab.

```text
$ ./read.py 鬱蒼たる樹海の中に舞う人の如き影が在った。
鬱蒼[うっそう]たる　樹海[じゅかい]　の　中[なか]　に　舞[ま]う
人[じん]　の　如[ごと]き　影[かげ]　が　在[あ]った　。
```

## reading_quiz/

A quiz asking kanji -> kana questions. Only works as an IRC plugin for now.

### Example IRC session

```text
<Christoph>  !quiz jlpt2
<nihongobot> Please read: 発見
<Christoph>  !quiz はっけん
<nihongobot> Christoph: Correct! (はっけん:
             (n,vs) 1. discovery, 2. detection, 3. finding)
```

## romaji/

`romaji.sh` converts kanji and kana to romaji using mecab.

```text
$ ./romaji.sh 鬱蒼たる樹海の中に舞う人の如き影が在った。
 ussoutaru jukai no naka ni mau jin no gotoki kage ga atta 。
```

## rtk/

`rtk.sh` looks up keywords, kanji and numbers. The keywords and numbers refer to
Heisig's amazing book "Remembering the Kanji".

```text
$ ./rtk.sh 城壁
#362: castle 城 | #1500: wall 壁

$ ./rtk.sh star
#1556: star 星, #237: stare 眺, #1476: starve 餓,
#2532: star-anise 樒, #2872: start 孟, #2376: mustard 芥

$ ./rtk.sh 1 2 3
#1: one 一 | #2: two 二 | #3: three 三
```

## simple_bot/

As the name says, this is a simple IRC bot. You can start it with:

```text
./bot.py <server[:port]> <channel> <nickname> [NickServ password]
```

It uses all the other scripts.
