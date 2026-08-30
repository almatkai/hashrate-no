/// <reference types="@raycast/api">

/* 🚧 🚧 🚧
 * This file is auto-generated from the extension's manifest.
 * Do not modify manually. Instead, update the `package.json` file.
 * 🚧 🚧 🚧 */

/* eslint-disable @typescript-eslint/ban-types */

type ExtensionPreferences = {
  /** API Key - API Key from Hashrate.no */
  "api_key": string
}

/** Preferences accessible in all the extension's commands */
declare type Preferences = ExtensionPreferences

declare namespace Preferences {
  /** Preferences accessible in the `coins` command */
  export type Coins = ExtensionPreferences & {}
  /** Preferences accessible in the `gpu_estimates` command */
  export type GpuEstimates = ExtensionPreferences & {}
}

declare namespace Arguments {
  /** Arguments passed to the `coins` command */
  export type Coins = {}
  /** Arguments passed to the `gpu_estimates` command */
  export type GpuEstimates = {
  /** POWER_COST (0-1.00) */
  "power_cost": string
}
}

