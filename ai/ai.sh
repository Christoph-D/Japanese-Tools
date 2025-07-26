#!/usr/bin/env bash
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# LLM query.

# shellcheck source=gettext/gettext.sh
. "$(dirname "$0")"/../gettext/gettext.sh

# shellcheck disable=SC1091
. "$(dirname "$0")"/api-keys

readonly DEEPSEEK_API_ENDPOINT=https://api.deepseek.com/v1/chat/completions
readonly OPENROUTER_API_ENDPOINT=https://openrouter.ai/api/v1/chat/completions

# Hardcoded limit on line length for IRC
readonly MAX_LINE_LENGTH=300

# Default system prompt. See system_prompt() for per-channel prompts.
DEFAULT_SYSTEM_PROMPT='You are a helpful AI in an IRC chatroom. You communicate with experienced software developers.
Write in English unless the user asks for something else. Keep your response under '"${MAX_LINE_LENGTH}"' characters.
Write only a single line. Your answers are suitable for all age groups.'

if [[ $LANG = de_DE.UTF-8 ]]; then
  DEFAULT_SYSTEM_PROMPT='Du bist eine hilfreiche KI in einem IRC-Chatraum. Du redest mit erfahrenen Software-Entwicklern.
Schreib auf Deutsch, außer wenn der User dich um etwas anderes bittet. Antworte mit maximal '"${MAX_LINE_LENGTH}"' Zeichen.
Schreib nur eine einzige Zeile. Deine Antworten sind für alle Altersstufen geeignet.'
fi

# Default model
api_endpoint=${DEEPSEEK_API_ENDPOINT}
api_key=${DEEPSEEK_API_KEY}
DEFAULT_MODEL=deepseek-chat
model=${DEFAULT_MODEL}

# Prevent usage in private messages
if [[ $IRC_PLUGIN = 1 && ${DMB_RECEIVER:0:1} != '#' ]]; then
    echo_ '!ai is only available in channels.'
    exit 1
fi

if [[ ! -v DEEPSEEK_API_KEY || ! -v OPENROUTER_API_KEY ]]; then
   echo_ 'No API keys available.'
   exit 1
fi

if [[ ! -v DEEPSEEK_MODELS || ! -v OPENROUTER_MODELS ]]; then
   echo_ 'No models available.'
   exit 1
fi

query=$*

list_models() {
    printf_ 'Usage: !ai [-model] <query>. Known models: %s %s. Default: %s\n' "${DEEPSEEK_MODELS}" "${OPENROUTER_MODELS}" "${DEFAULT_MODEL}"
}

# Load a per-channel system prompt if one exists.
# To use this feature, put a file with the same name as the IRC channel
# into channel_prompts, e.g.: channel_prompts/#mychannel
select_system_prompt() {
    system_prompt=${DEFAULT_SYSTEM_PROMPT}
    [[ -v DMB_RECEIVER && ${DMB_RECEIVER} =~ ^#[^./]*$ ]] || return
    prompt_file="$(dirname "$0")/channel_prompts/${DMB_RECEIVER}"
    [[ -f $prompt_file ]] || return
    system_prompt=$(cat "${prompt_file}")
}

select_model() {
    query_after_model_selection=${1}
    if [[ ${1:0:1} != - ]]; then
        return
    fi
    first_word=${1%% *}
    first_word=${first_word#-}
    query_after_model_selection=${1#* }
    for m in ${DEEPSEEK_MODELS}; do
        if [[ $m = "$first_word" ]]; then
            api_endpoint=${DEEPSEEK_API_ENDPOINT}
            api_key=${DEEPSEEK_API_KEY}
            model=$m
            return
        fi
    done
    for m in ${OPENROUTER_MODELS}; do
        if [[ $m = "$first_word" ]]; then
            api_endpoint=${OPENROUTER_API_ENDPOINT}
            api_key=${OPENROUTER_API_KEY}
            model=$m
            return
        fi
    done
    printf_ 'Unknown model. %s\n' "$(list_models)"
    return 1
}

json_escape() {
    # \ -> \\
    local s=${1//\\/\\\\}
    # " -> \"
    local s=${s//\"/\\\"}
    # \n -> \\n
    local s=${s//$'\n'/\\n}
    printf '%s' "$s"
}

query() {
    result=$(curl "${api_endpoint}" \
    --silent \
    -H "Authorization: Bearer $api_key" \
    -H "Content-Type: application/json" \
    -d '{
    "model": "'"${model}"'",
    "messages": [
        {
            "role": "system",
            "content": "'"$(json_escape "${system_prompt}")"'"
        },
        {
            "role": "user",
            "content": "'"$1"'"
        }
    ],
    "max_tokens": 300
    }' 2>&1)
    # shellcheck disable=SC2181
    if [[ $? -ne 0 ]]; then
        printf_ 'API error: %s' "${result}"
        return 1
    fi
    if ! result=$(printf '%s' "${result}" | python3 -c "import sys, json; sys.tracebacklimit = 0; print(json.load(sys.stdin)['choices'][0]['message']['content'])" 2>&1); then
        printf_ 'Invalid response: %s' "${result}"
        return 1
    fi
    printf '%s' "${result}"
}

sanitize_output() {
    local s=${1//$'\n'/}
    t=${s:0:$MAX_LINE_LENGTH}
    if [[ $s != "$t" ]]; then
        t="$t..."
    fi
    printf '%s' "$t"
}

if [[ -z $query ]]; then
    list_models
    exit
fi


if ! select_model "${query}"; then
    exit
fi

select_system_prompt

query=$(json_escape "${query_after_model_selection}")
result=$(query "${query}")
result=$(sanitize_output "${result}")

# Prevent triggering other bots that might be present in the same channel.
if [[ ${result:0:1} = '!' ]]; then
    printf ' '
fi
printf '%s\n' "${result}"
