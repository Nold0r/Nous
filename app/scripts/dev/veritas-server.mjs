import http from 'node:http';
import { pathToFileURL } from 'node:url';
import { Server } from 'socket.io';

const AVATARS = [
  { id: 1002, name: 'Dan Heng' },
  { id: 1003, name: 'Himeko' },
  { id: 1004, name: 'Welt' },
  { id: 1001, name: 'March 7th' },
];
const ATTACKS = [
  { type: 'Basic', skillType: 'Normal', name: 'Cloudlancer Art: North Wind' },
  { type: 'Skill', skillType: 'BPSkill', name: 'Molten Detonation' },
  { type: 'Ultimate', skillType: 'Ultra', name: 'Synthetic Black Hole' },
  { type: 'Follow-Up', skillType: 'Insert', name: 'Girl Power' },
  { type: 'DoT', skillType: 'DOT', name: 'Wind Shear' },
  { type: 'Break', skillType: 'ElementDamage', name: 'Weakness Break' },
];
const clone = value => structuredClone(value);
const event = (name, data) => ({ name, data });

/** Deterministic source-shaped packets, with no game or process dependency. */
export function createBattleFixture() {
  let active = false;
  let wave = 1;
  let cycle = 30;
  let turns = [];
  let avHistory = [];
  let totalDamage = 0;
  let enemies = [];
  let log = [];

  function record(events) {
    log.push(...clone(events));
    if (log.length > 4096) log.splice(0, log.length - 4096);
    return events;
  }

  function initializeWave() {
    const base = wave === 1 ? 9001 : 9003;
    enemies = [
      { id: 1001010, uid: base, name: 'Voidranger: Reaver', hp: wave === 1 ? 90000 : 130000 },
      { id: 1001020, uid: base + 1, name: 'Voidranger: Trampler', hp: wave === 1 ? 125000 : 150000 },
    ].map(enemy => ({ ...enemy, maxHP: enemy.hp }));
    return [
      event('OnUpdateWave', { wave }),
      ...enemies.map(enemy => event('OnInitializeEnemy', { enemy: {
        id: enemy.id, uid: enemy.uid, name: enemy.name,
        base_stats: { properties: { Level: 85, MaxHP: enemy.hp, CurrentHP: enemy.hp } },
      } })),
      formation(),
    ];
  }

  function formation() {
    return event('OnUpdateTeamFormation', {
      entities: enemies.filter(enemy => enemy.hp > 0).map(enemy => ({ uid: enemy.uid, team: 'Enemy' })),
      team: 'Enemy',
    });
  }

  function summary() {
    return {
      avatars: clone(AVATARS), turn_history: clone(turns), av_history: clone(avHistory),
      turn_count: turns.length, total_damage: totalDamage,
      action_value: turns.at(-1)?.action_value ?? 0, cycle, wave,
      // The pinned source never stores begin's stage_id in BattleContext.
      stage_id: 0,
    };
  }

  return {
    begin() {
      active = true;
      wave = 1;
      cycle = 30;
      turns = [];
      avHistory = [];
      totalDamage = 0;
      log = [];
      const events = [
        event('OnBattleBegin', { max_waves: 2, max_cycles: 30, stage_id: 100001 }),
        event('OnSetBattleLineup', { avatars: clone(AVATARS) }),
        event('OnUpdateTeamFormation', { entities: AVATARS.map(avatar => ({ uid: avatar.id, team: 'Player' })), team: 'Player' }),
        event('OnUpdateCycle', { cycle }),
        ...initializeWave(),
      ];
      for (const avatar of AVATARS) {
        for (const [type, value] of Object.entries({ CurrentHP: 4200, MaxHP: 4200, Attack: 2400, Defence: 1100, Speed: 134, ActionDelay: 74.6269 })) {
          events.push(event('OnStatChange', { entity: { uid: avatar.id, team: 'Player' }, property: { type, value } }));
        }
      }
      return record(events);
    },
    turn() {
      if (!active) return [];
      const index = turns.length;
      const events = [];
      if (enemies.every(enemy => enemy.hp <= 0) && wave < 2) {
        wave += 1;
        events.push(...initializeWave());
      }
      const avatarIndex = index % AVATARS.length;
      const avatar = AVATARS[avatarIndex];
      const attacker = { uid: avatar.id, team: 'Player' };
      const attack = ATTACKS[index % ATTACKS.length];
      const actionValue = Math.floor(index / 4) * 135 + Math.min(index % 4 + 1, 3) * 45;
      const nextCycle = 30 - Math.max(0, Math.floor((actionValue - 150) / 100) + 1);
      if (nextCycle !== cycle) {
        cycle = nextCycle;
        events.push(event('OnUpdateCycle', { cycle }));
      }
      events.push(
        event('OnTurnBegin', { action_value: actionValue, turn_owner: attacker }),
        event('OnUseSkill', { avatar: attacker, skill: {
          name: attack.name, type: attack.skillType, skill_config_id: avatar.id * 100 + index % 6 + 1,
        } }),
      );
      const damage = 18000 + index * 3250 + (attack.type === 'Ultimate' ? 30000 : 0) + (index === 11 ? 60000 : 0);
      const target = enemies.find(enemy => enemy.hp > 0) ?? enemies.at(-1);
      for (const hit of [damage * 0.4, damage * 0.6]) {
        const overkill = Math.max(0, hit - target.hp);
        target.hp = Math.max(0, target.hp - hit);
        events.push(
          event('OnDamage', { attacker, damage: hit, overkill_damage: overkill, type: attack.type }),
          event('OnStatChange', { entity: { uid: target.uid, team: 'Enemy' }, property: { type: 'CurrentHP', value: target.hp } }),
        );
      }
      if (target.hp === 0) events.push(formation());
      events.push(event('OnStatChange', { entity: attacker, property: { type: 'ActionDelay', value: 74.6269 } }));
      const avatarDamage = [0, 0, 0, 0];
      avatarDamage[avatarIndex] = damage;
      const turn = { action_value: actionValue, cycle, wave, avatars_turn_damage: avatarDamage, total_damage: damage };
      turns.push(clone(turn));
      const previous = avHistory.at(-1);
      if (previous?.action_value === actionValue) {
        previous.avatars_turn_damage = previous.avatars_turn_damage.map((value, index) => value + avatarDamage[index]);
        previous.total_damage += damage;
      } else avHistory.push(clone(turn));
      totalDamage += damage;
      events.push(event('OnTurnEnd', { turn_info: turn }));
      return record(events);
    },
    end() {
      if (!active) return [];
      active = false;
      return record([event('OnBattleEnd', summary())]);
    },
    get state() { return { active, ...summary(), enemies: clone(enemies) }; },
    get events() { return clone(log); },
  };
}

/** A local test instrument. HTTP controls remain available while Socket.IO is gated. */
export async function createVeritasServer({ port = 1305, host = '127.0.0.1', playIntervalMs = 1250 } = {}) {
  if (host !== '127.0.0.1') throw new Error('Veritas simulator only binds 127.0.0.1 loopback');
  let accepting = true;
  let timer;
  const fixture = createBattleFixture();
  const log = [];
  function remember(packet) {
    log.push(clone(packet));
    if (log.length > 4096) log.splice(0, log.length - 4096);
  }
  function broadcast(events) {
    for (const packet of events) {
      remember(packet);
      io.emit(packet.name, packet.data);
    }
  }
  function stopPlay() { clearInterval(timer); timer = undefined; }
  const httpServer = http.createServer(async (request, response) => {
    const path = new URL(request.url, 'http://127.0.0.1').pathname;
    function json(status, data) {
      response.writeHead(status, { 'Content-Type': 'application/json; charset=utf-8', 'Cache-Control': 'no-store' });
      response.end(JSON.stringify(data));
    }
    if (request.method === 'GET') {
      if (path === '/health') return json(200, { ok: true, accepting, active: fixture.state.active, port: httpServer.address().port, eventCount: log.length, playing: Boolean(timer) });
      if (path === '/events') return json(200, log);
      if (path === '/state') return json(200, fixture.state);
      return json(404, { error: 'Not found' });
    }
    if (request.method !== 'POST') return json(405, { error: 'Use POST for controls' });
    try {
      const body = await readJson(request);
      switch (path) {
        case '/begin': stopPlay(); broadcast(fixture.begin()); break;
        case '/turn': broadcast(fixture.turn()); break;
        case '/end': stopPlay(); broadcast(fixture.end()); break;
        case '/play': {
          const interval = body.intervalMs ?? playIntervalMs;
          const count = body.turns ?? 12;
          if (!Number.isInteger(interval) || interval < 1 || interval > 60000 || !Number.isInteger(count) || count < 1 || count > 100) {
            return json(400, { error: 'intervalMs must be 1..60000; turns must be 1..100' });
          }
          stopPlay();
          broadcast(fixture.begin());
          let played = 0;
          timer = setInterval(() => {
            broadcast(fixture.turn());
            played += 1;
            if (played >= count) { stopPlay(); broadcast(fixture.end()); }
          }, interval);
          break;
        }
        case '/disconnect': accepting = false; stopPlay(); io.disconnectSockets(true); break;
        case '/reconnect': accepting = true; break;
        case '/emit':
          if (typeof body.name !== 'string' || body.name.length === 0 || !Object.hasOwn(body, 'data')) return json(400, { error: 'Expected {name:string,data:any}' });
          broadcast([event(body.name, body.data)]);
          break;
        default: return json(404, { error: 'Not found' });
      }
      json(200, { ok: true, accepting, active: fixture.state.active, turnCount: fixture.state.turn_count });
    } catch (error) {
      json(400, { error: error.message });
    }
  });
  const io = new Server(httpServer, {
    cors: { origin: '*' },
    allowRequest: (_request, callback) => callback(null, accepting),
  });
  io.use((_socket, next) => accepting ? next() : next(new Error('Simulator disconnected')));
  io.on('connection', socket => {
    const packet = event('Connected', { version: '0.2.57' });
    remember(packet);
    socket.emit(packet.name, packet.data);
  });
  try {
    await new Promise((resolve, reject) => {
      httpServer.once('error', reject);
      httpServer.listen(port, host, resolve);
    });
  } catch (error) {
    io.close();
    throw error;
  }
  return {
    port: httpServer.address().port,
    url: `http://${host}:${httpServer.address().port}`,
    get state() { return fixture.state; },
    get events() { return clone(log); },
    close: () => {
      stopPlay();
      return new Promise(resolve => io.close(resolve));
    },
  };
}

async function readJson(request) {
  let body = '';
  for await (const chunk of request) {
    body += chunk;
    if (body.length > 65536) throw new Error('Control payload exceeds 64 KiB');
  }
  const parsed = body.trim() ? JSON.parse(body) : {};
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) throw new Error('Control body must be an object');
  return parsed;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const server = await createVeritasServer();
    console.log(`Veritas 0.2.57 simulator: ${server.url} (Socket.IO /socket.io/)`);
    console.log('POST /begin /turn /end /play /disconnect /reconnect /emit; GET /health /state /events');
    for (const signal of ['SIGINT', 'SIGTERM']) process.once(signal, () => server.close().then(() => process.exit(0)));
  } catch (error) {
    console.error(error.code === 'EADDRINUSE' ? '127.0.0.1:1305 is occupied; simulator did not replace the listener.' : error.message);
    process.exitCode = 1;
  }
}
