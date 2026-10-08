"""Check combat play numerically. Static warps and injected hit fixtures cannot pass.

SPDX-License-Identifier: GPL-3.0-only. All evidence stays in private/work/fight.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import struct
import zlib


def valid_png(path):
    if not path.is_file():
        return False
    data = path.read_bytes()
    if data[:8] != b'\x89PNG\r\n\x1a\n':
        return False
    offset, compressed, shape, end = 8, bytearray(), None, False
    try:
        while offset < len(data):
            count = struct.unpack_from('>I', data, offset)[0]
            tag = data[offset+4:offset+8]
            payload = data[offset+8:offset+8+count]
            crc = struct.unpack_from('>I', data, offset+8+count)[0]
            if len(payload) != count or zlib.crc32(tag+payload) != crc:
                return False
            if tag == b'IHDR':
                shape = struct.unpack('>IIBBBBB', payload)
            elif tag == b'IDAT':
                compressed.extend(payload)
            elif tag == b'IEND':
                end = True
            offset += 12+count
        if not end or not shape:
            return False
        width, height, depth, color, compression, filtering, interlace = shape
        if width < 320 or height < 224 or width > 8192 or height > 8192 or depth != 8 or color not in [2, 6] or compression or filtering or interlace:
            return False
        stride = width*(4 if color == 6 else 3)+1
        decoded = zlib.decompress(compressed)
        return len(decoded) == stride*height and all(decoded[i*stride] <= 4 for i in range(height))
    except (struct.error, ValueError, zlib.error):
        return False


def evaluate(log, exit_code, screenshot):
    def rows(label):
        result = []
        for line in log.splitlines():
            if line.startswith(label + ' '):
                pairs = re.findall(r'([a-zA-Z0-9_]+)=(-?\d+)', line)
                result.append({k: int(v) for k, v in pairs})
        return result
    players, npcs, hits = rows('COMBAT_PLAYER'), rows('COMBAT_NPC'), rows('COMBAT_HIT')
    checks = rows('CHECK')
    failures = []
    if 'FIGHT_FIXTURE ' in log:
        failures.append('isolated rendering fixture is not combat play')
    if exit_code or 'BLOCKED native service:' in log or len(checks) != 1 or checks[0].get('code') != 0:
        failures.append('native run did not finish without a guard')
    if 'MAP_WARP selected=' not in log:
        failures.append('missing explicit warp identity')
    if len(players) < 2 or any(b.get('tick', -1) <= a.get('tick', -1) for a, b in zip(players, players[1:])):
        failures.append('missing or unordered gameplay samples; a static warp is insufficient')
    if not any(a.get('health', 0) > b.get('health', 0) and a.get('health', 0) > 0 for a, b in zip(players, players[1:])):
        failures.append('Harry health never decreased')
    player_slot = 6  # Original NPC_COUNT_MAX; the player is the following identity.
    if not any(h.get('target') == player_slot and 0 <= h.get('attacker', -1) < player_slot and h.get('damage', 0) > 0 for h in hits):
        failures.append('no positive enemy-to-Harry production hit')
    killed = []
    for slot in range(player_slot):
        history = [n for n in npcs if n.get('slot') == slot]
        for before, after in zip(history, history[1:]):
            if before.get('id') == after.get('id') and before.get('health', 0) > 0 and after.get('health', 1) <= 0:
                if any(h.get('attacker') == player_slot and h.get('target') == slot and h.get('damage', 0) > 0
                       and before.get('tick', 0) <= h.get('tick', -1) <= after.get('tick', 0) for h in hits):
                    killed.append(slot)
    if not killed:
        failures.append('no continuous live enemy identity died from a positive Harry hit')
    if not valid_png(screenshot):
        failures.append('missing private PNG capture')
    return {'pass': not failures, 'failures': failures, 'player_samples': len(players),
            'npc_samples': len(npcs), 'production_hits': len(hits), 'killed_slots': sorted(set(killed))}


def self_test(evidence):
    path = evidence/'evaluator-control.png'
    chunk = lambda tag, data: struct.pack('>I', len(data))+tag+data+struct.pack('>I', zlib.crc32(tag+data))
    image = b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR', struct.pack('>IIBBBBB',320,224,8,2,0,0,0))
    image += chunk(b'IDAT',zlib.compress(bytes((320*3+1)*224)))+chunk(b'IEND',b'')
    path.write_bytes(image)
    log = '''MAP_WARP selected=MAP0_S00 spawn=0
COMBAT_PLAYER tick=1 health=100
COMBAT_NPC tick=1 slot=0 id=14 health=50
COMBAT_HIT tick=1 attacker=0 target=6 damage=10
COMBAT_PLAYER tick=2 health=90
COMBAT_HIT tick=2 attacker=6 target=0 damage=50
COMBAT_NPC tick=2 slot=0 id=14 health=0
CHECK code=0 frames=3 state=11 step=2
'''
    positive = evaluate(log,0,path)['pass']
    negative = [log.replace('health=90','health=100'),log.replace('damage=50','damage=0'),
                log.replace('health=0','health=1'),log.replace('id=14 health=0','id=3 health=0'),
                log+'BLOCKED native service: fixture',log.replace('MAP_WARP selected=','STATIC selected='),
                log.replace('tick=2 health=90','tick=1 health=90'),log+'FIGHT_FIXTURE tick=1',
                'MAP_WARP selected=MAP0_S00\nCHECK code=0\n']
    rejected = sum(not evaluate(text,0,path)['pass'] for text in negative)
    rejected += int(not evaluate(log,1,path)['pass'])
    path.write_bytes(image[:16])
    rejected += int(not evaluate(log,0,path)['pass'])
    path.unlink()
    result = {'pass':positive and rejected==11,'positive':int(positive),'rejected':rejected,'expected_rejections':11}
    (evidence/'evaluator-controls.json').write_text(json.dumps(result,indent=2))
    print(json.dumps(result))
    return int(not result['pass'])


def check_render_fixtures(evidence):
    results = []
    for mode in ['handgun', 'pipe']:
        log = (evidence/(mode+'.log')).read_text(encoding='utf-8')
        rows = [{k:int(v) for k,v in re.findall(r'(\w+)=(-?\d+)', line)}
                for line in log.splitlines() if line.startswith('FIGHT_FRAME ')]
        check = 'CHECK code=0 frames=3650 state=11 step=2' in log
        ordered = len(rows)==151 and [r.get('tick') for r in rows]==list(range(3500,3651))
        aim = sum(r.get('aim',0)!=0 for r in rows)
        attacks = sum(r.get('attacks',0)!=0 for r in rows)
        keys = len({r.get('extra_keyframe') for r in rows})
        held = max((r.get('weapon_draws',0) for r in rows),default=0)
        packets = max((r.get('fx_bytes',0) for r in rows),default=0)
        result = {'name':mode, 'fixture_only':True, 'samples':len(rows), 'aim_samples':aim,
                  'attack_samples':attacks, 'distinct_keyframes':keys, 'held_draw_calls':held, 'effect_packet_bytes':packets}
        result['pass'] = check and ordered and aim>60 and attacks>30 and keys>5 and held>30 and packets>0
        result['pass'] &= 'FIGHT_FIXTURE ' in log and 'FIGHT_WEAPON ' in log and 'BLOCKED native service:' not in log
        result['pass'] &= valid_png(evidence/(mode+'.png'))
        results.append(result)
    (evidence/'render-results.json').write_text(json.dumps(results,indent=2))
    print(json.dumps(results,indent=2))
    return int(not all(r['pass'] for r in results))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--log', type=Path)
    parser.add_argument('--screenshot', type=Path)
    parser.add_argument('--exit-code', type=int, default=0)
    parser.add_argument('--run', action='store_true')
    parser.add_argument('--self-test', action='store_true')
    parser.add_argument('--render-fixtures', action='store_true', help='verify isolated fixture logs; never a combat-play pass')
    parser.add_argument('--warp', default='MAP0_S00:0')
    parser.add_argument('--frames', type=int, default=10000)
    parser.add_argument('--input', type=Path, default=Path('docs/sys/replays/first_fight.txt'))
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    evidence = root.parents[1] / 'private/work/fight'
    evidence.mkdir(parents=True, exist_ok=True)
    if args.self_test:
        return self_test(evidence)
    if args.render_fixtures:
        return check_render_fixtures(evidence)
    log_path = args.log or evidence / 'combat.log'
    screenshot = args.screenshot or evidence / 'combat.png'
    if not screenshot.resolve().is_relative_to(evidence.parent.resolve()):
        parser.error('screenshots must stay under private/work')
    code = args.exit_code
    if args.run:
        if args.screenshot:
            parser.error('--run writes the fixed private/work/fight/combat.png capture')
        run = subprocess.run([str(root/'tools/dev-cargo.cmd'), 'test', '--release', '-p', 'silent-hill-boot',
                              'fight_warp', '--', '--ignored', '--nocapture', '--test-threads=1'],
                             cwd=root, env=dict(os.environ, SH_PLAYER_TRACE='1', SH_EVENTS_TRACE='1',
                                               SH_FIGHT_WARP=args.warp, SH_FIGHT_FRAMES=str(args.frames),
                                               SH_FIGHT_INPUT=str(args.input.resolve())),
                             capture_output=True, text=True, timeout=300)
        log_path.write_text(run.stdout+run.stderr, encoding='utf-8')
        code = run.returncode
    result = evaluate(log_path.read_text(encoding='utf-8'), code, screenshot)
    result.update(log=str(log_path), screenshot=str(screenshot), exit_code=code)
    (evidence/'combat-results.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
    print(json.dumps(result, indent=2))
    return int(not result['pass'])


if __name__ == '__main__':
    raise SystemExit(main())
