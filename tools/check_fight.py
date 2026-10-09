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
        # PORT: The original no-skip story is also a real combat entry route.
        # Require its movies, opening steps, text rollout and both alley doors.
        events = rows('EVENTS_FRAME')
        messages = rows('MESSAGE_FRAME')
        opening = {r.get('step0') for r in events if r.get('sys') == 10 and r.get('event') == 2}
        story = opening == set(range(15))
        for movie, count in [(2055, 338), (2056, 119)]:
            story &= bool(re.search(r'^MOVIE end id='+str(movie)+r' decoded='+str(count)+r' .* skipped=false completed=true$', log, re.M))
        for message in range(15, 20):
            history = [r for r in messages if r.get('id') == message]
            story &= any(0 < r.get('length', 0) < 400 for r in history)
            story &= not any(r.get('length') == 400 for r in history)
            story &= bool(re.search(r'^MESSAGE_PAGE tick=\d+ id='+str(message)+r'$', log, re.M))
        story &= 'AREA_TRANSITION tick=9552 map=MAP0_S00 room=26 sys=6 destination=0 point=19 trigger=18 flags=0' in log
        story &= bool(re.search(r'^AREA_TRANSITION tick=12901 .* point=22 trigger=20 ', log, re.M))
        if not story:
            failures.append('missing explicit warp or verified no-skip alley story')
    if len(players) < 2 or any(b.get('tick', -1) <= a.get('tick', -1) for a, b in zip(players, players[1:])):
        failures.append('missing or unordered gameplay samples; a static warp is insufficient')
    if not any(a.get('health', 0) > b.get('health', 0) and a.get('health', 0) > 0 for a, b in zip(players, players[1:])):
        failures.append('Harry health never decreased')
    player_slot = 6  # Original NPC_COUNT_MAX; the player is the following identity.
    if not any(h.get('target') == player_slot and 0 <= h.get('attacker', -1) < player_slot and h.get('damage', 0) > 0 for h in hits):
        failures.append('no positive enemy-to-Harry production hit')
    if not any(a.get('health', 0) > 0 and a.get('health', 0) > b.get('health', 0) and
               any(h.get('target') == player_slot and 0 <= h.get('attacker', -1) < player_slot and
                   h.get('damage', 0) > 0 and a.get('tick', 0) <= h.get('tick', -1) <= b.get('tick', 0)
                   and b.get('tick', 0) - h.get('tick', -1) <= 3 for h in hits)
               for a, b in zip(players, players[1:])):
        failures.append('Harry HP loss is not correlated with a production hit within three VBlanks')
    killed = []
    for slot in range(player_slot):
        history = [n for n in npcs if n.get('slot') == slot]
        for before, after in zip(history, history[1:]):
            if before.get('id') == after.get('id') and before.get('health', 0) > 0 and after.get('health', 1) <= 0:
                if any(h.get('attacker') == player_slot and h.get('target') == slot and h.get('damage', 0) > 0
                       and before.get('tick', 0) <= h.get('tick', -1) <= after.get('tick', 0)
                       and after.get('tick', 0) - h.get('tick', -1) <= 3 for h in hits):
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
    story = log.replace('MAP_WARP selected=MAP0_S00 spawn=0\n', '')
    story += 'MOVIE end id=2055 decoded=338 tick=20 skipped=false completed=true\nMOVIE end id=2056 decoded=119 tick=21 skipped=false completed=true\n'
    story += ''.join(f'EVENTS_FRAME tick={step+10} sys=10 event=2 step0={step}\n' for step in range(15))
    story += ''.join(f'MESSAGE_FRAME tick=30 id={message} length=20\nMESSAGE_PAGE tick=31 id={message}\n' for message in range(15,20))
    story += 'AREA_TRANSITION tick=9552 map=MAP0_S00 room=26 sys=6 destination=0 point=19 trigger=18 flags=0\n'
    story += 'AREA_TRANSITION tick=12901 map=MAP0_S00 point=22 trigger=20 flags=0\n'
    story_positive = evaluate(story,0,path)['pass']
    negative = [log.replace('health=90','health=100'),log.replace('damage=50','damage=0'),
                log.replace('health=0','health=1'),log.replace('id=14 health=0','id=3 health=0'),
                log+'BLOCKED native service: fixture',log.replace('MAP_WARP selected=','STATIC selected='),
                log.replace('tick=2 health=90','tick=1 health=90'),log+'FIGHT_FIXTURE tick=1',
                'MAP_WARP selected=MAP0_S00\nCHECK code=0\n',
                log.replace('tick=2 health=90','tick=10 health=90'),
                log.replace('COMBAT_NPC tick=2 slot=0','COMBAT_NPC tick=10 slot=0'),
                story.replace('decoded=338','decoded=337'),
                story.replace('step0=14','step0=13'),
                story.replace('id=15 length=20','id=15 length=400'),
                story.replace('point=22 trigger=20','point=23 trigger=20'),
                story.replace('MESSAGE_PAGE tick=31 id=15','MESSAGE_PAGE tick=31 id=14')]
    rejected = sum(not evaluate(text,0,path)['pass'] for text in negative)
    rejected += int(not evaluate(log,1,path)['pass'])
    path.write_bytes(image[:16])
    rejected += int(not evaluate(log,0,path)['pass'])
    path.unlink()
    result = {'pass':positive and story_positive and rejected==18,'positive':int(positive)+int(story_positive),'rejected':rejected,'expected_rejections':18}
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
    parser.add_argument('--warp', help='optional diagnostic entry; default follows the no-skip story')
    parser.add_argument('--frames', type=int, default=19650)
    parser.add_argument('--audio', default='off', help='host audio mode: off, on, or wav:private/work/PATH')
    parser.add_argument('--input', type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    from milestones import project_root
    evidence = project_root(root) / 'private/work/fight'
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
        replay = args.input or root / ('docs/sys/replays/first_fight.txt' if args.warp else 'docs/core/replays/opening_noskip.txt')
        command = [str(root/'target/release/silent-hill-boot.exe'), '--headless', '--audio', args.audio,
                   '--scale', '1', '--frames', str(args.frames), '--input', str(replay.resolve()),
                   '--screenshot', str(screenshot.resolve())]
        if args.warp:
            command += ['--warp', args.warp]
        env = {k: v for k, v in os.environ.items() if k not in ('SH_FIGHT_DIAGNOSTIC', 'SH_MAP_WARP', 'SH_FIGHT_WARP')}
        env.update(SH_PLAYER_TRACE='1', SH_EVENTS_TRACE='1')
        # PORT: Stream real samples to private evidence while the replay runs.
        # This preserves diagnostics if a guard/timeout interrupts the process.
        with log_path.open('w', encoding='utf-8') as sink:
            run = subprocess.run(command, cwd=root, env=env,
                                 stdout=sink, stderr=subprocess.STDOUT, timeout=300)
        code = run.returncode
    result = evaluate(log_path.read_text(encoding='utf-8'), code, screenshot)
    result.update(log=str(log_path), screenshot=str(screenshot), exit_code=code)
    result_path = log_path.with_suffix('.json') if args.log else evidence/'combat-results.json'
    result_path.write_text(json.dumps(result, indent=2), encoding='utf-8')
    print(json.dumps(result, indent=2))
    return int(not result['pass'])


if __name__ == '__main__':
    raise SystemExit(main())
