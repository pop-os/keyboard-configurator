#!/usr/bin/env python3

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from typing import List, Tuple, Dict

QMK_MAPPING = {
    'APPLICATION': 'APP',
    'AUDIO_MUTE': 'MUTE',
    'AUDIO_VOL_DOWN': 'VOLUME_DOWN',
    'AUDIO_VOL_UP': 'VOLUME_UP',
    'BSLASH': 'BACKSLASH',
    'BSLS': 'BACKSLASH',
    'BSPACE': 'BKSP',
    'BSPC': 'BKSP',
    'CAPSLOCK': 'CAPS',
    'COMM': 'COMMA',
    'DELETE': 'DEL',
    'DOT': 'PERIOD',
    'ENT': 'ENTER',
    'EQUAL': 'EQUALS',
    'EQL': 'EQUALS',
    'ESCAPE': 'ESC',
    'GRAVE': 'TICK',
    'GRV': 'TICK',
    'KP_0': 'NUM_0',
    'KP_1': 'NUM_1',
    'KP_2': 'NUM_2',
    'KP_3': 'NUM_3',
    'KP_4': 'NUM_4',
    'KP_5': 'NUM_5',
    'KP_6': 'NUM_6',
    'KP_7': 'NUM_7',
    'KP_8': 'NUM_8',
    'KP_9': 'NUM_9',
    'KP_ASTERISK': 'NUM_ASTERISK',
    'KP_COMMA': 'NUM_COMMA',
    'KP_DOT': 'NUM_PERIOD',
    'KP_ENTER': 'NUM_ENTER',
    'KP_EQUAL': 'NUM_EQUALS',
    'KP_MINUS': 'NUM_MINUS',
    'KP_PLUS': 'NUM_PLUS',
    'KP_SLASH': 'NUM_SLASH',
    'LALT': 'LEFT_ALT',
    'LBRACKET': 'BRACE_OPEN',
    'LBRC': 'BRACE_OPEN',
    'LCTRL': 'LEFT_CTRL',
    'LCTL': 'LEFT_CTRL',
    'LGUI': 'LEFT_SUPER',
    'LSHIFT': 'LEFT_SHIFT',
    'LSFT': 'LEFT_SHIFT',
    'NO': 'NONE',
    'MEDIA_NEXT_TRACK': 'MEDIA_NEXT',
    'MEDIA_PLAY_PAUSE': 'PLAY_PAUSE',
    'MEDIA_PREV_TRACK': 'MEDIA_PREV',
    'MINS': 'MINUS',
    'NUMLOCK': 'NUM_LOCK',
    'PGDOWN': 'PGDN',
    'PSCREEN': 'PRINT_SCREEN',
    'QUOT': 'QUOTE',
    'RALT': 'RIGHT_ALT',
    'RBRACKET': 'BRACE_CLOSE',
    'RBRC': 'BRACE_CLOSE',
    'RCTRL': 'RIGHT_CTRL',
    'RCTL': 'RIGHT_CTRL',
    'RGUI': 'RIGHT_SUPER',
    'RSHIFT': 'RIGHT_SHIFT',
    'RSFT': 'RIGHT_SHIFT',
    'SCOLON': 'SEMICOLON',
    'SCLN': 'SEMICOLON',
    'SLSH': 'SLASH',
    'SPC': 'SPACE',
    'SYSTEM_SLEEP': 'SUSPEND'
}

def extract_scancodes(ecdir: str, is_qmk: bool) -> List[Tuple[str, int]]:
    "Extract mapping from scancode names to numbers"

    if is_qmk:
        includes = [f"{ecdir}/tmk_core/common/keycode.h"]
        common_keymap_h = open(includes[0]).read()
        scancode_defines = re.findall(
            '    (KC_[^,\s]+)', common_keymap_h)
    else:
        includes = [f"{ecdir}/src/common/include/common/keymap.h"]
        common_keymap_h = open(includes[0]).read()
        scancode_defines = re.findall(
            '#define.*((?:K_\S+)|(?:KT_FN))', common_keymap_h)

    tmpdir = tempfile.mkdtemp()
    with open(f'{tmpdir}/keysym-extract.c', 'w') as f:
        f.write('#include <stdio.h>\n')
        f.write('int main() {\n')
        for i in scancode_defines:
            f.write(f'printf("%d ", {i});\n')
        f.write('}\n')

    cmd = ['gcc']
    for i in includes:
        cmd.append('-include')
        cmd.append(i)
    cmd += ['-o', f'{tmpdir}/keysym-extract', f'{tmpdir}/keysym-extract.c']
    subprocess.check_call(cmd)

    output = subprocess.check_output(
        f'{tmpdir}/keysym-extract', universal_newlines=True)

    shutil.rmtree(tmpdir)

    scancode_names = (i.split('_', 1)[1] for i in scancode_defines)
    if is_qmk:
        scancode_names = [QMK_MAPPING.get(i, i) for i in scancode_names]
    scancodes = (int(i) for i in output.split())
    scancode_list = list(zip(scancode_names, scancodes))
    
    if is_qmk:
        scancode_list.append(('FN', 0x5101)) # MO(0)
        scancode_list.append(('RESET', 0x5C00))
    else:
        scancode_list.append(('NONE', 0x0000))

    scancode_list = [(name, code) for (name, code) in scancode_list if name not in ('INT_1', 'INT_2')]

    # Make sure scancodes are unique
    assert len(scancode_list) == len(set(i for _, i in scancode_list))

    return scancode_list


def parse_layout_define(keymap_h: str, is_qmk) -> Tuple[List[str], List[List[str]]]:
    keymap_h = re.sub(r'/\*.*?\*/', '', keymap_h)
    # XXX split up regex?
    m = re.search(
        r'LAYOUT\((.*?)\)[\s\\]*({[^{}]*({[^{}]*}[^{}]*)+)[^{}]*}', keymap_h, re.MULTILINE | re.DOTALL)
    assert m is not None
    physical = m.group(1).replace(',', ' ').replace('\\', '').split()
    # XXX name?
    physical2 = [i.replace('\\', '').replace(',', '').split()
                 for i in m.group(2).replace('{', '').split('}')[:-1]]
    assert is_qmk or all(len(i) == len(physical2[0]) for i in physical2)
    return physical, physical2


def parse_keymap(keymap_c: str, physical: List[str], is_qmk: bool) -> Dict[str, List[str]]:
    # XXX for launch
    keymap_c = keymap_c.replace('MO(1)', 'FN')
    keymap_c = re.sub(r'/\*.*?\*/', '', keymap_c)

    layer_scancodes: List[List[str]] = []
    for layer in re.finditer(r'LAYOUT\((.*?)\)', keymap_c, re.MULTILINE | re.DOTALL):
        scancodes = layer.group(1).replace(',', ' ').split()
        assert len(scancodes) == len(physical)

        def scancode_map(x: int, code: str) -> str:
            if code == '0':
                return 'NONE'

            code = code.replace('K_', '').replace('KC_', '').replace('KT_', '')

            if is_qmk:
                code = QMK_MAPPING.get(code, code)

                # Handle TRNS
                if layer_scancodes and code == 'TRNS':
                    code = layer_scancodes[-1][x]

            return code

        scancodes = [scancode_map(x, i) for x, i in enumerate(scancodes)]
        layer_scancodes.append(scancodes)

    keymap = {}
    for i, physical_name in enumerate(physical):
        keymap[physical_name] = [j[i] for j in layer_scancodes]

    return keymap


def gen_layout_json(path: str, physical: List[str], physical2: List[List[str]]) -> None:
    "Generate layout.json file"

    layout = {}
    for p in physical:
        x, y = next((x, y) for x, i in enumerate(physical2)
                    for y, j in enumerate(i) if j == p)
        layout[p] = (x, y)

    with open(path, 'w') as f:
        json.dump(layout, f, indent=2)


def gen_keymap_json(path: str, scancodes: List[Tuple[str, int]]) -> None:
    "Generate keymap.json file"

    with open(path, 'w') as f:
       json.dump(scancodes, f, indent=2)


def gen_default_json(path: str, board: str, keymap: Dict[str, List[str]]) -> None:
    "Generate default.json file"

    with open(path, 'w') as f:
        json.dump({"board": board, "map": keymap}, f, indent=2)


def generate_layout_dir(ecdir: str, board: str, is_qmk: bool) -> None:
    layoutdir = f'layouts/{board}'
    print(f'Generating {layoutdir}...')

    if is_qmk:
        keymap_h = open(
            f"{ecdir}/keyboards/{board}/{board.split('/')[-1]}.h").read()
        default_c = open(
            f"{ecdir}/keyboards/{board}/keymaps/default/keymap.c").read()
    else:
        keymap_h = open(
            f"{ecdir}/src/board/{board}/include/board/keymap.h").read()
        default_c = open(f"{ecdir}/src/board/{board}/keymap/default.c").read()

    os.makedirs(f'{layoutdir}', exist_ok=True)

    physical, physical2 = parse_layout_define(keymap_h, is_qmk)
    scancodes = extract_scancodes(ecdir, is_qmk)
    default_keymap = parse_keymap(default_c, physical, is_qmk)
    gen_layout_json(f'{layoutdir}/layout.json', physical, physical2)
    gen_keymap_json(f'{layoutdir}/keymap.json', scancodes)
    gen_default_json(f'{layoutdir}/default.json', board, default_keymap)


parser = argparse.ArgumentParser()
parser.add_argument("ecdir")
parser.add_argument("board")
parser.add_argument("--qmk", action="store_true")
args = parser.parse_args()

if args.board == 'all':
    if args.qmk:
        boarddir = f'{args.ecdir}/keyboards/system76'
    else:
        boarddir = f'{args.ecdir}/src/board/system76'
    for i in os.listdir(boarddir):
        if i == 'common' or not os.path.isdir(f'{boarddir}/{i}'):
            continue
        generate_layout_dir(args.ecdir, f'system76/{i}', args.qmk)
else:
    generate_layout_dir(args.ecdir, args.board, args.qmk)
