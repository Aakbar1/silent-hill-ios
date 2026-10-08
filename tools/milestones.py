"""Headless milestone gate. Logs and screenshots always stay outside the repo."""
from pathlib import Path
from datetime import datetime, timezone
import argparse
import hashlib
import json
import re
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--available", action="store_true", help="run implemented milestones; does not certify the brief's pass")
    parser.add_argument("--repeat", type=int, default=1, choices=range(1, 4), help="compare PNG hashes between isolated runs")
    parser.add_argument("--disc", type=Path)
    parser.add_argument("--only", nargs="+", help="debug a named subset of available milestones")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    replays = root / "docs/core/replays"
    manifest = json.loads((replays / "milestones.json").read_text())
    if manifest["version"] != 1:
        raise SystemExit("unsupported milestone manifest version")
    if manifest["pending"] and not args.available:
        print("FAIL: full brief milestone gate is incomplete:", flush=True)
        for pending in manifest["pending"]:
            print(" - " + pending, flush=True)
        print("Run --available to verify completed work; it cannot certify first-map walking.", flush=True)
        return 2
    cases = manifest["verified"]
    if args.only:
        unknown = set(args.only) - {case["name"] for case in cases}
        if unknown:
            raise SystemExit("unknown milestones: " + ", ".join(sorted(unknown)))
        cases = [case for case in cases if case["name"] in args.only]
    private = root.parent.parent / "private/work/core5"
    # PORT: Lane workers can keep all replay captures in their own private tree.
    import os
    lane = os.environ.get("SH_MILESTONE_LANE")
    if lane:
        if lane not in ("player","move","events"):
            raise SystemExit("unsupported private milestone lane")
        private = root.parent.parent / ("private/work/"+lane)
    output = private / "milestones" / datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    output.mkdir(parents=True)
    executable = root / "target/release/silent-hill-boot.exe"
    flags = {"state":"--expect-state", "step":"--expect-step", "menu":"--expect-menu", "option_entry":"--expect-option-entry", "movie_skips":"--expect-movie-skips", "min_movie_frames":"--min-movie-frames", "min_lit_pixels":"--min-lit-pixels"}
    results = []
    passed = 0
    for case in cases:
        first_hash = None
        for repeat in range(args.repeat):
            stem = case["name"] + f"-{repeat+1}"
            screenshot = output / (stem + ".png")
            command = [str(executable), "--headless", "--audio", "off", "--frames", str(case["frames"]), "--input", str(replays / case["input"]), "--screenshot", str(screenshot)]
            if args.disc:
                command += ["--disc", str(args.disc.resolve())]
            for key, flag in flags.items():
                if key in case:
                    command += [flag, str(case[key])]
            try:
                run = subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=180)
            except subprocess.TimeoutExpired as error:
                def decoded(value):
                    return value.decode(errors="replace") if isinstance(value, bytes) else value or ""
                run = subprocess.CompletedProcess(command, -1, decoded(error.stdout), decoded(error.stderr) + "\nFAIL: native replay timed out after 180 seconds\n")
            (output / (stem + ".log")).write_text(run.stdout + run.stderr)
            checkpoint = [line for line in run.stdout.splitlines() if line.startswith("CHECK ")]
            digest = hashlib.sha256(screenshot.read_bytes()).hexdigest() if screenshot.exists() else None
            ok = run.returncode == 0 and len(checkpoint) == 1 and digest is not None
            # Audio's original VAB reads delay the intro start to VBlank 1338.
            # Retain cadence evidence alongside the measured frame threshold;
            # a delayed start or fewer decoded frames must still fail the gate.
            for movie_id, tick in case.get("movie_start_ticks", {}).items():
                begin = re.findall(r"MOVIE begin id=" + movie_id + r" .* tick=(\d+)$", run.stdout, re.M)
                ok &= begin == [str(tick)]
            if "movie_frame_count" in case:
                count = re.search(r"\bmovie_frames=(\d+)\b", checkpoint[0]) if checkpoint else None
                ok &= count is not None and int(count[1]) == case["movie_frame_count"]
            if repeat and digest != first_hash:
                ok = False
            first_hash = first_hash or digest
            results.append({"case":case["name"], "run":repeat+1, "exit":run.returncode, "checkpoint":checkpoint, "png_sha256":digest, "pass":ok})
            print(f"{'PASS' if ok else 'FAIL'} {stem}: " + (checkpoint[0] if checkpoint else "no checkpoint"), flush=True)
            if not ok:
                (output / "results.json").write_text(json.dumps(results, indent=2))
                print("Evidence: " + str(output), flush=True)
                return 1
            passed += 1
    (output / "results.json").write_text(json.dumps(results, indent=2))
    print(f"PASS available suite: {passed} runs; {len(cases) if args.repeat > 1 else 0} deterministic PNG comparisons. Full brief remains incomplete.", flush=True)
    print("Evidence: " + str(output), flush=True)
    return 0


def check_player_trace(log):
    """Check actual per-frame input, movement and camera evidence; never a screenshot."""
    import math
    import re
    if "BLOCKED native service:" in log or "CHECK code=0 frames=3720 state=11" not in log:
        raise ValueError("original replay did not finish 3720 frames in gameplay")
    rows = {}
    for line in log.splitlines():
        if line.startswith("PLAYER_FRAME "):
            row = {key: int(value) for key, value in re.findall(r"(\w+)=(-?\d+)", line)}
            fields = {"tick","state","sys","x","y","z","heading","camera_x","camera_z","look_x","look_z","lock_x","lock_z","pad"}
            if set(row) != fields:
                raise ValueError("malformed player frame")
            if row["tick"] in rows:
                raise ValueError("duplicate player frame")
            rows[row["tick"]] = row
    required = range(3299, 3721)
    if any(tick not in rows for tick in required):
        raise ValueError("missing per-frame player/camera evidence")
    if any(rows[tick]["state"] != 11 or rows[tick]["sys"] != 0 for tick in required):
        raise ValueError("movement window is outside controllable gameplay")
    def distance(a, b, x="x", z="z"):
        return math.hypot(b[x]-a[x], b[z]-a[z])
    def heading_delta(a, b):
        return (b["heading"]-a["heading"]+2048) % 4096 - 2048
    # Input is read between the first and second presentation after each replay
    # edge; the completed original run shows the new held mask at edge + 2.
    phases = [(3302,3419,0x10),(3452,3509,0x20),(3542,3689,0x8010)]
    for start,end,pad in phases:
        if any(rows[tick]["pad"] & 0xffff != pad for tick in range(start,end+1)):
            raise ValueError("pad replay was not consumed as requested")
    walk_angle = rows[3301]["heading"]*2*math.pi/4096
    walk_x,walk_z = rows[3419]["x"]-rows[3301]["x"],rows[3419]["z"]-rows[3301]["z"]
    walking = walk_x*math.sin(walk_angle)+walk_z*math.cos(walk_angle)
    sideways = walk_x*math.cos(walk_angle)-walk_z*math.sin(walk_angle)
    if walking < 4096 or abs(sideways) > max(512,walking/4):
        raise ValueError("forward walking did not follow Harry's measured heading")
    if abs(heading_delta(rows[3301],rows[3419])) > 8:
        raise ValueError("straight walking unexpectedly changed heading")
    turn = sum(heading_delta(rows[tick-1],rows[tick]) for tick in range(3452,3510))
    if turn <= 64 or turn >= 2048:
        raise ValueError("right turn did not change heading in the expected direction")
    if distance(rows[3451],rows[3509]) > 2048:
        raise ValueError("turn-only phase unexpectedly translated Harry")
    walk_speed = distance(rows[3360],rows[3419])/59
    run_speed = distance(rows[3620],rows[3689])/69
    if run_speed <= walk_speed*1.3:
        raise ValueError("run speed did not exceed walking speed")
    angle = rows[3600]["heading"]*2*math.pi/4096
    dx,dz = rows[3689]["x"]-rows[3600]["x"],rows[3689]["z"]-rows[3600]["z"]
    if dx*math.sin(angle)+dz*math.cos(angle) <= 4096:
        raise ValueError("running displacement did not follow the turned heading")
    # Original camera update precedes player update, so permit one frame of lag.
    for tick in required:
        row = rows[tick]
        previous = rows.get(tick-1,row)
        if min(math.hypot(row["lock_x"]-p["x"],row["lock_z"]-p["z"]) for p in (row,previous)) > 1024:
            raise ValueError("camera character target stopped following Harry")
        if math.hypot(row["look_x"]-row["x"],row["look_z"]-row["z"]) > 20*4096:
            raise ValueError("camera look target is detached from Harry")
    for start,end,_ in (phases[0],phases[2]):
        if distance(rows[start],rows[end],"look_x","look_z") < 1024:
            raise ValueError("camera look target stayed fixed while Harry moved")
        if distance(rows[start],rows[end],"camera_x","camera_z") < 1024:
            raise ValueError("camera position target stayed fixed while Harry moved")
    return {"walk_q12":walking,"right_turn_q12":turn,"walk_per_frame":walk_speed,"run_per_frame":run_speed,"sampled_frames":len(required)}


def first_map_main():
    import os
    parser = argparse.ArgumentParser(description="Original first-map player/camera milestone")
    parser.add_argument("--first-map", action="store_true")
    parser.add_argument("--scale", type=int, choices=range(1, 5), default=1, help="numeric replay render scale; game default is unchanged")
    parser.add_argument("--disc", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    lane=os.environ.get('SH_MILESTONE_LANE','move')
    if lane not in ('player','move','events'):raise SystemExit('unsupported private milestone lane')
    output = root.parent.parent / f"private/work/{lane}/milestones" / datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    output.mkdir(parents=True)
    command = [str(root/"target/release/silent-hill-boot.exe"),"--headless","--audio","off","--frames","3720","--input",str(root/"docs/core/replays/first_map.txt"),"--expect-state","11"]
    command += ["--scale",str(args.scale)]
    if args.disc:
        command += ["--disc",str(args.disc.resolve())]
    env = dict(os.environ,SH_PLAYER_TRACE="1")
    env.pop("SH_SOUND_STUB_BGM",None)
    try:
        run = subprocess.run(command,cwd=root,env=env,capture_output=True,text=True,timeout=180)
    except subprocess.TimeoutExpired as error:
        def decoded(value):
            return value.decode(errors="replace") if isinstance(value, bytes) else value or ""
        run = subprocess.CompletedProcess(command,-1,decoded(error.stdout),decoded(error.stderr)+"\nFAIL: replay timed out\n")
    log = run.stdout+run.stderr
    (output/"first-map.log").write_text(log,encoding="utf-8")
    result = {"exit":run.returncode,"pass":False}
    result["boundary"] = [line for line in log.splitlines() if line.startswith(("BLOCKED ","CHECK "))]
    try:
        if run.returncode:
            raise ValueError("native replay stopped before the movement checks")
        result.update(check_player_trace(log),pass_=True)
        result["pass"] = result.pop("pass_")
    except ValueError as error:
        result["reason"] = str(error)
    if result["pass"]:
        # PORT: Capture an actual forward-walking frame in a fresh replay;
        # numeric movement remains independently checked by the complete run.
        screenshot = output / "mid-walk.png"
        capture_command = command.copy()
        capture_command[capture_command.index("--frames")+1] = "3380"
        capture_command += ["--screenshot",str(screenshot)]
        capture = subprocess.run(capture_command,cwd=root,env=env,capture_output=True,text=True,timeout=180)
        (output/"mid-walk.log").write_text(capture.stdout+capture.stderr,encoding="utf-8")
        result["screenshot"] = str(screenshot)
        expected_frame = [line for line in log.splitlines() if line.startswith("PLAYER_FRAME tick=3380 ")]
        captured_frame = [line for line in capture.stdout.splitlines() if line.startswith("PLAYER_FRAME tick=3380 ")]
        if capture.returncode or not screenshot.exists() or len(expected_frame)!=1 or captured_frame!=expected_frame:
            result.update(pass_=False,reason="mid-walk capture failed")
            result["pass"] = result.pop("pass_")
    (output/"results.json").write_text(json.dumps(result,indent=2),encoding="utf-8")
    print(("PASS" if result["pass"] else "FAIL")+" first-map: "+json.dumps(result),flush=True)
    print("Evidence: "+str(output),flush=True)
    return 0 if result["pass"] else 1


def check_opening_trace(log):
    """Require the real full movie, gradual timed dialogue and restored control."""
    if "BLOCKED native service:" in log or "CHECK code=0 frames=4600 state=11 step=2" not in log:
        raise ValueError("unskipped opening did not finish in gameplay")
    if "MOVIE end id=2055 decoded=338" not in log or not re.search(r"MOVIE end id=2055 .* skipped=false completed=true",log):
        raise ValueError("opening movie was skipped or incomplete")
    events=[{key:int(value) for key,value in re.findall(r"(\w+)=(-?\d+)",line)} for line in log.splitlines() if line.startswith("EVENTS_FRAME ")]
    steps={row["step0"] for row in events if row["sys"]==10 and row["event"]==2}
    if steps!=set(range(15)):
        raise ValueError("opening callback did not execute every original step")
    messages=[{key:int(value) for key,value in re.findall(r"(\w+)=(-?\d+)",line)} for line in log.splitlines() if line.startswith("MESSAGE_FRAME ")]
    for message in range(15,20):
        rows=[row for row in messages if row["id"]==message]
        if not rows or not any(0<row["length"]<400 for row in rows) or not re.search(r'MESSAGE_PAGE tick=\d+ id='+str(message)+r'$',log,re.M):
            raise ValueError(f"message {message} has no rollout/completion evidence")
        if any(row["length"]==400 for row in rows):
            raise ValueError(f"message {message} was fast-forwarded")
    restored=[row for row in events if row["tick"]>4300 and row["sys"]==0 and row["flags"]==0 and row["npc0"]==28]
    if not restored or any(row["health"]!=409600 for row in restored):
        raise ValueError("opening did not restore gameplay/Cheryl without injected damage")
    return {"opening_movie_frames":338,"opening_steps":len(steps),"messages":5,"restored_control_tick":restored[0]["tick"]}


def check_combat_trace(log):
    """Require native queued hits and observed HP loss in each direction."""
    if 'BLOCKED native service:' in log or 'CHECK code=0 frames=11000 state=11' not in log:
        raise ValueError('first-area replay did not finish without a native guard')
    def rows(prefix):
        return [{key:int(value) for key,value in re.findall(r'(\w+)=(-?\d+)',line)} for line in log.splitlines() if line.startswith(prefix)]
    players=rows('COMBAT_PLAYER ')
    npcs=rows('COMBAT_NPC ')
    hits=rows('COMBAT_HIT ')
    player_drops=[current for previous,current in zip(players,players[1:]) if current['tick']==previous['tick']+1 and previous['health']>0 and current['health']<previous['health']]
    previous_npcs={}
    enemy_drops=[]
    for row in npcs:
        key=(row['slot'],row['id'])
        previous=previous_npcs.get(key)
        if previous and row['tick']==previous['tick']+1 and 1<row['id']<=24 and previous['health']>0 and row['health']<previous['health']:
            enemy_drops.append(row)
        previous_npcs[key]=row
    hurting=[hit for hit in hits if 0<=hit['attacker']<6 and hit['target']==6 and hit['damage']>0]
    fighting=[hit for hit in hits if hit['attacker']==6 and 0<=hit['target']<6 and hit['damage']>0]
    if not any(0<=drop['tick']-hit['tick']<=3 for drop in player_drops for hit in hurting):
        raise ValueError('no original enemy hit followed by Harry HP loss')
    if not any(drop['slot']==hit['target'] and 0<=drop['tick']-hit['tick']<=3 for drop in enemy_drops for hit in fighting):
        raise ValueError('no original Harry hit followed by enemy HP loss')
    return {'enemy_hits':len(hurting),'player_hits':len(fighting),'player_hp_drops':len(player_drops),'enemy_hp_drops':len(enemy_drops)}


def events_main():
    import os
    parser=argparse.ArgumentParser(description="Events lane original progression milestones")
    group=parser.add_mutually_exclusive_group(required=True)
    group.add_argument('--opening-noskip',action='store_true')
    group.add_argument('--first-area-combat',action='store_true')
    parser.add_argument('--disc',type=Path)
    parser.add_argument('--scale',type=int,choices=range(1,5),default=1)
    args=parser.parse_args()
    root=Path(__file__).resolve().parents[1]
    name='opening_noskip' if args.opening_noskip else 'first_area_combat'
    frames=4600 if args.opening_noskip else 11000
    output=root.parent.parent/'private/work/events/milestones'/datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
    output.mkdir(parents=True)
    command=[str(root/'target/release/silent-hill-boot.exe'),'--headless','--audio','off','--scale',str(args.scale),'--frames',str(frames),'--input',str(root/'docs/core/replays'/f'{name}.txt'),'--expect-state','11']
    if args.disc:command+=['--disc',str(args.disc.resolve())]
    env=dict(os.environ,SH_PLAYER_TRACE='1',SH_EVENTS_TRACE='1')
    env.pop('SH_SOUND_STUB_BGM',None)
    run=subprocess.run(command,cwd=root,env=env,capture_output=True,text=True,timeout=180)
    log=run.stdout+run.stderr
    (output/f'{name}.log').write_text(log,encoding='utf-8')
    result={'exit':run.returncode,'pass':False,'boundary':[line for line in log.splitlines() if line.startswith(('CHECK ','BLOCKED '))]}
    try:
        if run.returncode:raise ValueError('native progression stopped before milestone completion')
        if args.opening_noskip:result.update(check_opening_trace(log))
        else:result.update(check_combat_trace(log))
        result['pass']=True
    except ValueError as error:result['reason']=str(error)
    (output/'results.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
    print(('PASS' if result['pass'] else 'FAIL')+' '+name+': '+json.dumps(result),flush=True)
    print('Evidence: '+str(output),flush=True)
    return int(not result['pass'])


if __name__ == "__main__":
    sys.exit(events_main() if any(flag in sys.argv for flag in ('--opening-noskip','--first-area-combat')) else first_map_main() if "--first-map" in sys.argv else main())
