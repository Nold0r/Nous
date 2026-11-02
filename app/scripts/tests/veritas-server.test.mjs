import assert from 'node:assert/strict';
import { once } from 'node:events';
import { test } from 'node:test';
import { io } from 'socket.io-client';
import { createBattleFixture, createVeritasServer } from '../dev/veritas-server.mjs';

function eventData(events, name) {
  const event = events.find(event => event.name === name);
  assert.ok(event, `fixture emits ${name}`);
  return event.data;
}

test('fixture follows source 0.2.57 names, numeric payloads and truthful final damage', () => {
  const fixture = createBattleFixture();
  const begin = fixture.begin();
  assert.deepEqual(begin.slice(0, 2).map(event => event.name), ['OnBattleBegin', 'OnSetBattleLineup']);
  assert.equal(eventData(begin, 'OnSetBattleLineup').avatars.length, 4);
  assert.deepEqual(eventData(begin, 'OnBattleBegin'), { max_waves: 2, max_cycles: 30, stage_id: 100001 });
  assert.ok(eventData(begin, 'OnInitializeEnemy').enemy.base_stats.properties.MaxHP > 0);
  let damage = 0;
  const categories = new Set();
  let overkill = 0;
  for (let index = 0; index < 12; index++) {
    const events = fixture.turn();
    const turn = eventData(events, 'OnTurnEnd').turn_info;
    assert.equal(turn.avatars_turn_damage.length, 4);
    assert.equal(turn.total_damage, turn.avatars_turn_damage.reduce((sum, value) => sum + value, 0));
    const skill = eventData(events, 'OnUseSkill').skill;
    assert.equal(typeof skill.skill_config_id, 'number');
    assert.equal(typeof skill.type, 'string');
    for (const event of events.filter(event => event.name === 'OnDamage')) {
      assert.equal(event.data.attacker.team, 'Player');
      assert.equal(typeof event.data.attacker.uid, 'number');
      assert.equal(typeof event.data.damage, 'number');
      assert.equal(typeof event.data.overkill_damage, 'number');
      categories.add(event.data.type);
      damage += event.data.damage;
      overkill += event.data.overkill_damage;
    }
  }
  const end = eventData(fixture.end(), 'OnBattleEnd');
  assert.equal(end.turn_count, 12);
  assert.equal(end.total_damage, damage);
  assert.equal(end.turn_history.length, 12);
  assert.equal(end.avatars.length, 4);
  assert.ok(end.av_history.length > 0);
  assert.ok(categories.size >= 4);
  assert.ok(overkill > 0);
  assert.equal(end.wave, 2);
  assert.ok(end.cycle < 30);
  assert.deepEqual(fixture.end(), [], 'repeat end is a no-op');
});

const waitFor = (socket, name) => Promise.race([
  once(socket, name).then(([data]) => data),
  new Promise((_, reject) => setTimeout(() => reject(new Error(`Timeout waiting for ${name}`)), 1000).unref()),
]);

for (const transport of ['polling', 'websocket']) {
  test(`loopback server delivers actual Socket.IO ${transport} events and HTTP controls`, async t => {
    const server = await createVeritasServer({ port: 0 });
    t.after(() => server.close());
    const healthResponse = await fetch(`${server.url}/health`);
    assert.equal(healthResponse.status, 200);
    assert.equal((await healthResponse.json()).accepting, true);
    const socket = io(server.url, { transports: [transport], reconnection: false });
    t.after(() => socket.close());
    const connected = await waitFor(socket, 'Connected');
    assert.deepEqual(connected, { version: '0.2.57' });
    async function post(path, body) {
      return fetch(`${server.url}${path}`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body ?? {}) });
    }
    const lineup = waitFor(socket, 'OnSetBattleLineup');
    assert.equal((await post('/begin')).status, 200);
    assert.equal((await lineup).avatars.length, 4);
    const damage = waitFor(socket, 'OnDamage');
    await post('/turn');
    assert.ok((await damage).damage > 0);
    const ended = waitFor(socket, 'OnBattleEnd');
    await post('/end');
    assert.equal((await ended).turn_count, 1);
    const malformed = waitFor(socket, 'OnDamage');
    await post('/emit', { name: 'OnDamage', data: { broken: true } });
    assert.deepEqual(await malformed, { broken: true });
    const disconnected = once(socket, 'disconnect');
    await post('/disconnect');
    await disconnected;
    assert.equal((await (await fetch(`${server.url}/health`)).json()).accepting, false);
    const denied = io(server.url, { transports: [transport], reconnection: false, timeout: 500 });
    t.after(() => denied.close());
    await waitFor(denied, 'connect_error');
    await post('/reconnect');
    const resumed = io(server.url, { transports: [transport], reconnection: false });
    t.after(() => resumed.close());
    assert.deepEqual(await waitFor(resumed, 'Connected'), { version: '0.2.57' });
  });
}

test('server rejects a non-loopback bind without opening a listener', async () => {
  await assert.rejects(createVeritasServer({ port: 0, host: '0.0.0.0' }), /127\.0\.0\.1|loopback/);
});

test('occupied port fails without disturbing its existing server', async t => {
  const first = await createVeritasServer({ port: 0 });
  t.after(() => first.close());
  await assert.rejects(createVeritasServer({ port: first.port }), { code: 'EADDRINUSE' });
  assert.equal((await fetch(`${first.url}/health`)).status, 200);
});

test('timed play completes truthfully even after rejecting an invalid play request', async t => {
  const server = await createVeritasServer({ port: 0, playIntervalMs: 20 });
  t.after(() => server.close());
  const socket = io(server.url, { transports: ['websocket'], reconnection: false });
  t.after(() => socket.close());
  await waitFor(socket, 'Connected');
  const finished = waitFor(socket, 'OnBattleEnd');
  const post = body => fetch(`${server.url}/play`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) });
  assert.equal((await post({ turns: 3 })).status, 200);
  assert.equal((await post({ turns: 0 })).status, 400);
  const summary = await finished;
  assert.equal(summary.turn_count, 3);
  assert.equal(summary.total_damage, 93750);
  assert.equal(server.state.active, false);
  assert.equal(server.events.filter(packet => packet.name === 'OnDamage').reduce((sum, packet) => sum + packet.data.damage, 0), 93750);
  assert.ok(server.events.some(packet => packet.name === 'OnBattleEnd'));
  assert.equal((await (await fetch(`${server.url}/health`)).json()).playing, false);
});
