/** ******************************************************************************
 *  (c) 2020 ZondaX GmbH
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 ******************************************************************************* */
const Zemu = require("@zondax/zemu").default;
const Resolve = require("path").resolve;
const LogListen = require("@ledgerhq/logs").listen

// const TransportNodeHid = require('@ledgerhq/hw-transport-node-hid').default;
const signer_wasm = require('@zondax/filecoin-signer');


const seed = "equip will roof matter pink blind book anxiety banner elbow sun young"
const SIM_OPTIONS = {
    logging: true,
    start_delay: 4000,
    X11: true,
    custom: `-s "${seed}" --color LAGOON_BLUE`
};

before(async () => {
    process.on("SIGINT", () => {
        Zemu.stopAllEmuContainers(function () {
            process.exit();
        });
    });
    await Zemu.checkAndPullImage();
})

after(async () => {
    await Zemu.stopAllEmuContainers();
})

describe.skip("Ledger device", function () {
  it("Get version", async function () {
      this.timeout(10000);

      const transport = await TransportNodeHid.create();
      var answer = await signer_wasm.getVersionFromDevice(transport);

      console.log(answer);
  });

  it("Get address", async function () {
      this.timeout(10000);
      const path = "m/44'/461'/5/0/3";

      const transport = await TransportNodeHid.create()
      var answer = await signer_wasm.keyRetrieveFromDevice(path, transport);

      console.log(answer);
  });

  it("Show address", async function () {
      this.timeout(10000);
      const path = "m/44'/461'/5/0/3";

      const transport = await TransportNodeHid.create()
      var answer = await signer_wasm.showKeyOnDevice(path, transport);

      console.log(answer);
  });
})


// FIXME: I am confused why this is the "legacy" version when it seems to be the newest one
it.skip("With Zemu", async function () {
    this.timeout(10000);

    const DEMO_APP_PATH = Resolve("bin/app.elf");
    const sim = new Zemu(DEMO_APP_PATH);
    await sim.start(SIM_OPTIONS);

    const transport = sim.getTransport()

    // Subscribe to transport events to see what is going on...
    LogListen( (s) => {
        console.log(s);
    } )

    var answer = await signer_wasm.getVersion(transport);
    console.log(answer);
    // FIXME: The reply is not correct. Ideally, we need to return the error code, etc
    // We should be as backwards compatible as possible to what we already have

});

it.skip("With Zemu get address", async function () {
  this.timeout(10000);

  const DEMO_APP_PATH = Resolve("bin/app.elf");
  const sim = new Zemu(DEMO_APP_PATH);
  await sim.start(SIM_OPTIONS);

  const transport = sim.getTransport()
  const path = "m/44'/461'/5/0/3";

  // Subscribe to transport events to see what is going on...
  LogListen( (s) => {
      console.log(s);
  } )

  var answer = await signer_wasm.keyRetrieveFromDevice(path, transport);
  console.log(answer);
  // FIXME: The reply is not correct. Ideally, we need to return the error code, etc
  // We should be as backwards compatible as possible to what we already have

})

it.skip("With Zemu show address", async function () {
  this.timeout(10000);

  const DEMO_APP_PATH = Resolve("bin/app.elf");
  const sim = new Zemu(DEMO_APP_PATH);
  await sim.start(SIM_OPTIONS);

  const transport = sim.getTransport()
  const path = "m/44'/461'/5/0/3";

  // Subscribe to transport events to see what is going on...
  LogListen( (s) => {
      console.log(s);
  } )

  const respRequest = signer_wasm.showKeyOnDevice(path, transport);
  await Zemu.sleep(2000);

  // click right
  await sim.clickRight();
  await sim.clickRight();
  await sim.clickRight();
  await sim.clickRight();
  await sim.clickBoth();

  const answer = await respRequest;
  console.log(answer);

})

it("With Zemu signature", async function () {
  this.timeout(50000);

  const DEMO_APP_PATH = Resolve("bin/app.elf");
  const sim = new Zemu(DEMO_APP_PATH);
  await sim.start(SIM_OPTIONS);

  const transport = sim.getTransport()
  const path = "m/44'/461'/5/0/3";

  const message = Buffer.from(
    "885501fd1d0f4dfcd7e99afcb99a8326b7dc459d32c6285501b882619d46558f3d9e316d11b48dcf211327025a0144000186a0430009c4430061a80040",
    "hex",
  );

  // Subscribe to transport events to see what is going on...
  LogListen( (s) => {
      console.log(s);
  } )

  const answer = await signer_wasm.transactionSignRawWithDevice(message, path, transport);

  console.log(answer);

})
