// delivery/beacon-no-personal-field: the beacon carries no field that names a person or a connection (no email, IP, cookie, token, user or address field at any depth),
// and no value of the run's Access settings, no token, no e-mail-shaped text. The run's own Access values are read from the run, not written here.
import { defineScenario } from '../../lib/scenario.mjs';
import { beaconOf, live, personalNames } from '../../lib/delivery-tests.mjs';

export default defineScenario({
  id: 'delivery/beacon-no-personal-field',
  covers: ['delivery/beacon/no-personal-field'],
  displays: ['4k-dpr1'],
  async run({ page }) {
    const b = await beaconOf(page);
    if (b.text === null) return { ok: false, notes: ['no beacon was sent in 45 s'] };
    const d = live();
    const names = personalNames(JSON.parse(b.text));
    const values = [d.accessEnv.ACCESS_TEAM_DOMAIN, d.accessEnv.ACCESS_AUD].filter((v) => b.text.includes(v)).length;
    const jwt = /eyJ[A-Za-z0-9_-]{10,}/.test(b.text);
    const mail = /[\w.+-]+@[\w-]+\.[\w.]+/.test(b.text);
    const ok = names.length === 0 && values === 0 && !jwt && !mail;
    return { ok, notes: [`personal field names ${names.length}; run's Access values ${values}; token-shaped ${jwt}; e-mail-shaped ${mail}`], results: { names, accessValues: values, token: jwt, mail } };
  },
});
