#!/usr/bin/python3
# -*- coding: utf-8 -*-
# Copyright: Christoph Dittmann <github@christoph-d.de>
# License: GNU GPL, version 3 or later; http://www.gnu.org/copyleft/gpl.html
#
# A very simple IRC bot providing a single interface for most of the
# Japanese tools.
# 

from irc.bot import SingleServerIRCBot
from irc.client import ip_numstr_to_quad, ip_quad_to_numstr
import gettext
import string
import random
import os, subprocess, sys
import time
import io
import traceback
import locale
_ = gettext.gettext

scripts = [('ai', '../ai/ai'),
           ('cdecl', '../cdecl/c.sh'),
           ('c++decl', '../cdecl/c++.sh'),
           ('rtk', '../rtk/rtk.sh'),
           ('romaji', '../romaji/romaji.sh'),
           ('kanjidic', '../kanjidic/kanjidic.sh'),
           ('kana', '../reading/read.py'),
           ('hira', '../kana/hira.sh'),
           ('kata', '../kana/kata.sh'),
           (['ja','jp'], '../jmdict/jm.sh'),
           (['wa','wadoku'], '../jmdict/wa.sh'),
           ('audio', '../audio/find_audio.sh'),
           ('quiz', '../reading_quiz/quiz.sh'),
           ('kuiz', '../kumitate_quiz/kuiz.sh'),
           ('calc', '../mueval/run.sh'),
           ('type', '../mueval/type.sh'),
           ('utf', '../compare_encoding/compare_encoding.sh'),
           ('lhc', '../lhc/lhc_info.sh')
           ]

def run_script(path, argument, irc_source_target, ignore_errors=False):
    try:
        env = os.environ
        lang = env.get('LANG', 'en_US.utf8')
        env.update({ 'DMB_SENDER'   : irc_source_target[0],
                     'DMB_RECEIVER' : irc_source_target[1],
                     'LANGUAGE'     : lang,
                     'LANG'         : lang,
                     'LC_ALL'       : lang,
                     'IRC_PLUGIN'   : '1' })
        output = subprocess.Popen(
            [path, argument],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=os.path.dirname(os.path.abspath(path)),
            env=env
            ).communicate()[0]
        return output.decode('utf-8')
    except:
        if ignore_errors:
            return ''
        else:
            return _('An error occured.')

def limit_length(s, max_bytes):
    """Limits the length of a unicode string after conversion to
    utf-8. Returns a unicode string."""
    for limit in range(max_bytes, 0, -1):
        if len(s[:limit].encode('utf-8')) <= max_bytes:
            return s[:limit]
    return ''

class SimpleBot(SingleServerIRCBot):
    def __init__(self, channels, nickname, nickpass, server, port=6667):
        SingleServerIRCBot.__init__(self, [(server, port)], nickname, nickname)
        self.initial_channels = channels
        self.nickpass = nickpass
        # magic_key is used for admin commands. E.g., "magic_key say
        # test" in a query with the bot triggers the admin command
        # "say".
        self.magic_key = ''.join([random.choice(string.ascii_letters) for x in range(8)]) + ' '
        self.print_magic_key()
        self.current_topic = ''
        self._timers = []
        self._connect()

    def print_magic_key(self):
        print(_('Today\'s magic key for admin commands: %s') % self.magic_key, end=' ')
        sys.stdout.flush()

    def debug_out(self, line):
        # Overwrite magic key.
        print('\r' + (60 * ' ') + '\r' + line)
        # Print magic key again.
        self.print_magic_key()

    def say(self, lines, to=None):
        if to is None:
            to = self.say_target
        # Limit maximum number of lines and line length.
        for line in lines.splitlines()[:4]:
            self.connection.privmsg(to, limit_length(line, 410))

    def on_nicknameinuse(self, c, e):
        c.nick(c.get_nickname() + '_')

    def on_welcome(self, c, e):
        if self.nickpass is not None:
            c.privmsg('NickServ', 'identify ' + self.nickpass)
        # Set bot mode.
        c.mode(c.get_nickname(), '+B')
        for channel in self.initial_channels:
            c.join(channel)

    def on_privmsg(self, c, e):
        self.current_event = e
        self.say_target = e.source.nick
        line = e.arguments[0]
        if len(line) > 0 and line[0] == '!':
            line = line[1:]
        self.do_command(line)
        self.debug_out('<%s> %s' % (e.source, line))

    def on_pubmsg(self, c, e):
        self.current_event = e
        a = e.arguments[0]
        if len(a) > 0 and a[0] == '!':
            self.say_target = e.target
            self.do_command(a[1:])
        return

    def do_command(self, cmd):
        """This method will never raise an exception based on the
        Exception base class."""
        try:
            self.do_command_unsafe(cmd)
        except Exception as e:
            output = io.StringIO()
            output.write(_('Caught exception: %s\n') % str(e))
            traceback.print_exc(file = output)
            self.debug_out(output.getvalue())
            output.close()

    def do_command_unsafe(self, cmd):
        """This method could raise an exception."""
        if cmd[0:len(self.magic_key)] == self.magic_key:
            self.do_special_command(cmd[len(self.magic_key):])
        else:
            self.do_user_command(cmd)

    def do_special_command(self, cmd):
        """Commands only the admin may use."""
        cmd = cmd.split(' ', 1)
        if cmd[0] == 'die':
            if len(cmd) == 1:
                self.die('さようなら')
            else:
                self.die(cmd[1])
        elif cmd[0] == 'join':
            self.connection.join(cmd[1])
        elif cmd[0] == 'part':
            self.connection.part(cmd[1])
        elif cmd[0] == 'raw':
            self.connection.send_raw(cmd[1])
        elif cmd[0] == 'privmsg':
            cmd = cmd[1].split(' ', 1)
            self.connection.privmsg(cmd[0], cmd[1])
        else:
            self.say(_('Unknown command.'))

    def get_source_target(self):
        e = self.current_event
        source = e.source.nick
        target = e.target
        if target == self.connection.get_nickname():
            return (source, source)
        else:
            return (source, target)

    def do_user_command(self, cmd):
        """Commands normal users may use."""
        if cmd == 'version':
            return self.say(_('A very simple bot with 日本語 support.'))
        elif cmd == 'help':
            return self.show_help()
        split_pos = cmd.find(' ')
        split_pos2 = cmd.find('　')
        split_pos_len = len(' ')
        if split_pos == -1 or (split_pos2 != -1 and split_pos2 < split_pos):
            split_pos = split_pos2
            split_pos_len = len('　')
        if split_pos == -1:
            cmd = [cmd, '']
        else:
            cmd = [cmd[:split_pos], cmd[split_pos + split_pos_len:]]
        e = self.current_event
        for s in scripts:
            name = s[0]
            if type(name) is not list:
                name = [name]
            if cmd[0] in name:
                output = run_script(s[1], cmd[1], self.get_source_target())
                self.handle_script_output(output, s[1])

    def handle_script_output(self, output, script):
        result = []
        for line in output.split('\n'):
            if not line.startswith('/timer '):
                result.append(line)
            else:
                args = line.split(' ')
                self.add_timer(int(args[1]), script, args[2])
        self.say('\n'.join(result))

    def show_help(self):
        possible_commands = [ '!' + str(s[0]) for s in scripts ] + ['!version']
        possible_commands.sort()
        self.say(_('Known commands: ') + ', '.join(possible_commands))

    def add_timer(self, delay_seconds, script, argument):
        """Adds a new timer."""
        e = self.current_event
        timer = (delay_seconds + time.time(), script, argument,
                 self.get_source_target(), self.say_target)
        self._timers.append(timer)

    def run_timed_command(self, timer):
        """Runs the command associated with the timer."""
        self.say_target = timer[4]
        self.handle_script_output(run_script(timer[1], timer[2], timer[3]), timer[1])

    def check_timers(self):
        current_time = time.time()
        # Check for expired timers.
        [ self.run_timed_command(t) for t in self._timers if t[0] < current_time ]
        # Remove expired timers.
        self._timers = [ t for t in self._timers if t[0] >= current_time ]

    def next_word_of_the_day(self, old_word):
        file_done = 'word_of_the_day_done.txt'
        file_next = 'word_of_the_day_next.txt'
        try:
            f = open(file_next, 'r')
            f2 = open(file_next + '.tmp', 'w')
            next_word = f.readline()
            for line in f:
                f2.write(line)
            f.close()
            f2.close()
            os.rename(file_next + '.tmp', file_next)
        except IOError:
            next_word = ''
        if next_word:
            f = open(file_done, 'a')
            f.write(old_word + '\n')
            f.close()
        return next_word.strip()
    def daily_jobs(self):
        """This method will be called once per day a few seconds after
        midnight."""
        marker = 'Wort des Tages: '
        if self.current_topic.find(marker) != -1:
            prefix, old_word = self.current_topic.split(marker, 1)
            if old_word.find(' ') != -1:
                print('old_word: ' + old_word)
                old_word, suffix = old_word.split(' ', 1)
                suffix = ' ' + suffix
                print('suffix: ' + suffix)
            else:
                suffix = ''
            new_word = self.next_word_of_the_day(old_word)
            if new_word:
                new_topic = '%s%s%s%s' % (prefix, marker, new_word, suffix)
                print('new topic: [%s]' % new_topic)
                self.connection.topic(self.initial_channels[0], new_topic)

    def polling_jobs(self):
        if self.connection.is_connected():
            pass

    def check_daily_jobs(self):
        current_time = time.strftime('%a')
        if hasattr(self, 'daily_jobs_last_time') and current_time != self.daily_jobs_last_time:
            self.daily_jobs()
        self.daily_jobs_last_time = current_time

    def check_polling_jobs(self):
        current_time = time.time()
        if hasattr(self, 'polling_jobs_last_time') and current_time != self.polling_jobs_last_time:
            self.polling_jobs()
        self.polling_jobs_last_time = current_time

    def on_currenttopic(self, c, e):
        if e.arguments[0] == self.initial_channels[0]:
            self.current_topic = e.arguments[1]
    def on_topic(self, c, e):
        if e.target == self.initial_channels[0]:
            self.current_topic = e.arguments[0]

    def run_forever(self):
        """In order to support custom timers, we can't call
        self.start()."""
        while True:
            self.check_timers()
            self.ircobj.process_once(0.2)
            self.check_daily_jobs()
            self.check_polling_jobs()

def setup_gettext():
    gettext.bindtextdomain('japanese_tools', '../gettext/locale')
    gettext.textdomain('japanese_tools')

def main():
    # Set preferred locale.
    locale.setlocale(locale.LC_ALL, '')

    # Change working directory to the location of this script so we
    # can work with relative paths.
    os.chdir(sys.path[0])

    setup_gettext()

    if len(sys.argv) != 4 and len(sys.argv) != 5:
        print(_('Usage: bot.py <server[:port]> <channel[,channel...]> <nickname> [NickServ password]'))
        sys.exit(1)

    s = sys.argv[1].split(':', 1)
    server = s[0]
    if len(s) == 2:
        try:
            port = int(s[1])
        except ValueError:
            print(_('Error: Invalid port.'))
            sys.exit(1)
    else:
        port = 6667
    channels = sys.argv[2].split(",")
    nickname = sys.argv[3]
    nickpass = None
    if len(sys.argv) == 5:
        nickpass = sys.argv[4]

    bot = SimpleBot(channels, nickname, nickpass, server, port)
    try:
        bot.run_forever()
    except KeyboardInterrupt:
        print(_('Caught KeyboardInterrupt, exiting...'))
        bot.do_special_command('die')
        bot.start()

if __name__ == '__main__':
    main()
