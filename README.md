# Japanese learning tools / IRC bot

A collection of scripts and tools to help learn Japanese, including dictionary
lookups, quizzes, and an IRC bot.

## Table of contents

- [Installation](#installation)
- [Usage](#usage)
- [AI assistant](#ai-assistant)
- [Word and kanji lookup](#word-and-kanji-lookup)
- [Kanji readings](#kanji-readings)
- [Quizzes](#quizzes)
- [Other tools](#other-tools)
- [IRC bot](#irc-bot)

## Installation

### Development Container

For the quickest setup, use the provided devcontainer which includes all
dependencies pre-installed:

1. Open in VS Code with the Dev Containers extension
2. Select "Reopen in Container" when prompted
3. Run `./build.sh` to build binaries, translations, and dictionaries
4. [optional, slow] Run `./install_mueval.sh` to install the `mueval` tool

### Manual Installation

These tools have been tested on Ubuntu 24.04 and later. Install dependencies:

```bash
# Required dependencies
sudo apt install gettext mecab-jumandic-utf8 mecab libssl-dev kakasi xmlstarlet xsltproc sqlite3 libsqlite3-dev bc liburi-perl build-essential pkg-config python3 wget
cargo install xtr
# Optional, required only for lhc
sudo apt install tesseract-ocr imagemagick
# Optional, required only for cdecl
sudo apt install cdecl
# Optional, required only for mueval
sudo apt install libffi-dev libffi8ubuntu1 libgmp-dev libgmp10 libncurses-dev
./install_mueval.sh
```

The last command installs [GHCup](https://www.haskell.org/ghcup/install/) and
[mueval](https://hackage.haskell.org/package/mueval). If you don't use the
devcontainer, you need to run `source ~/.ghcup/env` to set up `PATH`, for
example in `.bashrc`.

To build binaries, translations, and dictionaries:

```bash
./build.sh
```

## Usage

The scripts can be used as plugins in [the IRC bot](#irc-bot) or run directly in
the shell. Useful aliases:

```bash
alias ja="$JAPANESE_TOOLS/jmdict/jm.sh"
alias wa="$JAPANESE_TOOLS/jmdict/wa.sh"
alias rtk="$JAPANESE_TOOLS/rtk/rtk.sh"
```

## AI assistant

Query an LLM for Japanese language help or any other question. Supports
different providers, including OpenRouter, Mistral, Deepseek, LiteLLM. You need
put at least one API key into `.env`.

```bash
cd ai
cp .env.example .env
# Edit .env to add your API keys and model config
cp config.toml.example config.toml
# Edit config.toml to configure the available models
cargo run -- "What's 夜空?"
```

```text
$ cd ai
$ cargo run
Usage: !ai [-g|-o|-p|-m] [-clear_history|-c] [-temp=1.0|-t=1.0] <query>.  Models: [g]Gemini 2.5 Flash [o]GPT-4o mini [p]GPT-4.1 [m]Mistral Medium.  Default: Deepseek v3 0324
$ cargo run -- "What's 夜空?"
"夜空" means "night sky" in English. It refers to the sky as seen at night, often with stars and the moon.
$ cargo run -- "-g What's 夜空? Also give me a Japanese example sentence."
[g] Yozora (夜空) means night sky. 今夜は夜空が綺麗ですね。(The night sky is beautiful tonight.)
```

### Example of short-term memory

```text
<Christoph>  !ai What is 夜空?
<nihongobot> 夜空 means "night sky" in Japanese, referring to the sky as seen after sunset.
<Christoph>  !ai Do you know any other words like this?
<nihongobot> Yes! 星空 (hoshi-zora) means "starry sky" and 夕空 (yuu-zora) means "evening sky."
<Christoph>  !ai Something for morning?
<nihongobot> 朝空 (asa-zora) means "morning sky" in Japanese.
```

## Word and kanji lookup

### Audio

Find audio pronunciations for Japanese words from languagepod101.

```text
$ ./audio/find_audio.sh 夜空
Audio for 夜空 [よぞら]: http://tinyurl.com/p8aq8jo
```

### JMDict/Wadoku

Look up Japanese words in JMDict (Japanese-English) and Wadoku (Japanese-German)
dictionaries. Best for Japanese->English/German lookups, not the opposite
direction.

Requires running `prepare_jmdict.sh` and `prepare_wadoku.sh` first to download
and process the dictionaries.

```text
$ ./jmdict/jm.sh 村長
村長 [そんちょう,むらおさ] (n), 1. village headman, 2. village mayor
市長村長選挙 [しちょうそんちょうせんきょ] (n), mayoral election
```

### Kanjidic

Look up kanji information including stroke count, readings, and meanings.

```text
$ ./kanjidic/kanjidic.sh 日本語
日: 4 strokes. ニチ, ジツ, ひ, -び, -か. In names: あ, あき, いる, く, くさ, こう, す, たち, に, にっ, につ, へ {day, sun, Japan, counter for days}
本: 5 strokes. ホン, もと. In names: まと, ごう {book, present, main, origin, true, real, counter for long cylindrical things}
語: 14 strokes. ゴ, かた.る, かた.らう {word, speech, language}
```

### RTK

Look up kanji by keyword, kanji, or Heisig's "Remembering the Kanji" numbers.

```text
$ ./rtk/rtk.sh 城壁
#362: castle 城 | #1500: wall 壁

$ ./rtk/rtk.sh star
#1556: star 星, #237: stare 眺, #1476: starve 餓, #2532: star-anise 樒, #2872: start 孟, #2376: mustard 芥

$ ./rtk/rtk.sh 1 2 3
#1: one 一 | #2: two 二 | #3: three 三
```

## Kanji readings

### Kana conversion

Convert kanji to kana using mecab.

```text
$ ./reading/read.py 鬱蒼たる樹海の中に舞う人の如き影が在った。
鬱蒼[うっそう]たる　樹海[じゅかい]　の　中[なか]　に　舞[ま]う
人[じん]　の　如[ごと]き　影[かげ]　が　在[あ]った　。
```

### Romaji conversion

Convert kanji and kana to romaji using mecab.

```text
$ ./romaji/romaji.sh 鬱蒼たる樹海の中に舞う人の如き影が在った。
 ussoutaru jukai no naka ni mau jin no gotoki kage ga atta 。
```

## Quizzes

### Kana trainer

Practice hiragana and katakana recognition.

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

### Kumitate quiz

Tests your understanding of Japanese sentence structure by asking you to arrange
words in the correct order.

```text
<Flamerokz> !kuiz skm2
<nihongobot> Please choose [1-4]: 周囲の人たちの　＿　＿　★　＿　と思う。 (1: 協力を 2: 優勝は 3: 無理だった 4: 抜きにしては).
<Flamerokz> !kuiz 2
<nihongobot> Flamerokz: Correct! (2: 優勝は)
```

**Question file format:**

For `!kuiz` to work, you need to put one or more question files with a `.txt`
suffix into `kumitate_quiz/questions/`. The files need to contain lines of the
following form:

```text
周囲の人たちの　＿　＿　★　＿　と思う。|協力を,優勝は,無理だった,抜きにしては|2
```

Each line contains three parts separated by `|`:

1. The sentence with blanks (represented by `＿` and `★`)
2. The choices separated by commas
3. The correct answer number (1-4)

**Caveats:**

- Requires question files in `kumitate_quiz/questions/` with `.txt` suffix
- Currently only works as an IRC plugin

### Reading quiz

Tests your ability to read kanji by asking for kana readings.

```text
<Christoph>  !quiz jlpt2
<nihongobot> Please read: 発見
<Christoph>  !quiz はっけん
<nihongobot> Christoph: Correct! (はっけん:
             (n,vs) 1. discovery, 2. detection, 3. finding)
```

**Question file format:**

For `!quiz` to work, you need to put one or more vocabulary files with a `.txt`
suffix into `reading_quiz/vocabulary/`. The files need to contain lines of the
following form:

```text
発見|はっけん|(n,vs) 1. discovery, 2. detection, 3. finding
```

Each line contains three parts separated by `|`:

1. The kanji word to read
2. The correct kana reading
3. The meaning/definition

**Caveats:**

- Requires vocabulary files in `reading_quiz/vocabulary/` with `.txt` suffix
- Currently only works as an IRC plugin

## Other tools

### Gettext internationalization

Provides internationalization support for the tools.

**Supported languages:**

- English (default)
- German
- Polish

**Usage:**

1. Run `gettext/extract_strings.sh` to extract strings from the source code.
1. Translate the strings in `gettext/po/<language>.po`.
1. Run `gettext/regenerate_mo_files.sh` to use the translations.

### Compare encoding

Compares the size of different encodings for Japanese text.

```text
$ ./compare_encoding.sh 夜空
UTF-8 vs. UTF-16: 91213 vs. 156876 bytes. UTF-8 wins by 41.8%.
```

### LHC status

Displays the current status of the Large Hadron Collider at CERN (unrelated to
Japanese learning).

```bash
./lhc/lhc_info.sh
```

## IRC bot

A simple IRC bot that integrates all the tools above.

```bash
cd ircbot
cargo run -- <server[:port]> <channel> <nickname> [NickServ password]
```

## Tokenizer

Tokenize text using the Deepseek V3 or Llama3 tokenizer, for exploring LLMs.

```text
$ cd tokenizer
$ cargo run --  '世界は美しくなんかない。そしてそれ故に、美しい'
"世界", "は", "美", "し", "くな", "ん", "かない", "。", "そして", "それ", "故", "に", "、", "美", "しい"
$ cargo run --  '-llama3 世界は美しくなんかない。そしてそれ故に、美しい'
"世界", "は", "美", "しく", "なん", "かない", "。そして", "それ", "故", "に", "、", "美", "しい"
```
