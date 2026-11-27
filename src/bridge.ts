import {invoke, isTauri} from '@tauri-apps/api/core';
import {listen} from '@tauri-apps/api/event';
export const desktop = isTauri();
export interface Settings {schemaVersion:number;dark:boolean;auto:boolean;rebuild:boolean;localized:boolean;sourcePath:string|null}
export interface Installation {path:string;version:string;architecture:string;kind:string;layout:string}
export interface Snapshot {settings:Settings;installations:Installation[];selected:Installation|null;running:boolean;warnings:string[];dataDir:string;platform:string}
export interface Asset {platform:string;architecture:string;name:string;version:string;size:number;sha256:string}
export interface Release {version:string;tag:string;checkedAt:number;assets:Asset[]}
export interface DownloadTask {id:string;stage:string;assetName:string;version:string;downloaded:number;total:number;sha256:string;path:string|null;error:string|null;cancellable:boolean}
export interface DownloadState {channel:{accelerated:boolean;repository:string};release:Release|null;task:DownloadTask|null;lastError:string|null;retryAt:number;stale:boolean}
export interface PackageInfo {name:string;publisher:string;version:string;architecture:string;signatureStatus:string;signerMatches:boolean}
export interface InstallState {id:string;stage:string;taskId:string;info:PackageInfo|null;message:string;startedAt:number;checkedAt:number;needsRebuild:boolean;targetPath?:string|null;backupPath?:string|null;stagingPath?:string|null}
export interface RuntimeState {managedRunning:boolean;originalRunning:boolean;unknownRunning:boolean}
export interface LocalState {stage:string;message:string;copied:number;total:number;sourceVersion:string;sourcePath:string|null;managedExe:string|null;backup:string|null;patchVersion:number}
export interface BackupEntry {id:string;version:string;valid:boolean;message:string;bytes:number;modified:number;kind:string}
export interface RecoveryState {entries:BackupEntry[];pending:boolean;transactionMessage:string;currentValid:boolean;currentMessage:string}
export const bridge = {
  snapshot:()=>invoke<Snapshot>('get_app_snapshot'),
  save:(settings:Settings)=>invoke<Settings>('update_settings',{settings}),
  select:()=>invoke<Settings|null>('select_source_installation'),
  launch:()=>invoke<void>('launch_official'),
  openData:()=>invoke<void>('open_data_directory'),
  openLink:(kind:'source'|'release')=>invoke<void>('open_project_link',{kind}),
  channel:(accelerated:boolean)=>invoke<DownloadState>('configure_download_channel',{accelerated}),
  downloads:()=>invoke<DownloadState>('get_download_state'),
  checkRelease:(manual:boolean)=>invoke<DownloadState>('check_release',{manual}),
  download:(platform:string,architecture:string)=>invoke<DownloadTask>('start_download',{platform,architecture}),
  cancel:(taskId:string)=>invoke<void>('cancel_download',{taskId}),
  verify:()=>invoke<DownloadTask>('verify_download'),
  reveal:()=>invoke<void>('reveal_download'),
  onTask:(callback:(task:DownloadTask)=>void)=>listen<DownloadTask>('download-task',event=>callback(event.payload)),
  installation:()=>invoke<InstallState>('get_install_state'),
  prepareInstall:()=>invoke<InstallState>('prepare_install'),
  launchInstall:(installId:string,forceExit=false)=>invoke<InstallState>('launch_install',{installId,forceExit}),
  recheckInstall:()=>invoke<InstallState>('recheck_install'),
  stopInstall:()=>invoke<InstallState>('stop_install_wait'),
  localizationRuntime:()=>invoke<RuntimeState>('get_localization_runtime'),
  localization:()=>invoke<LocalState>('get_localization_state'),
  prepareLocalized:(forceExit=false)=>invoke<LocalState>('prepare_localized_copy',{forceExit}),
  cancelLocalized:()=>invoke<void>('cancel_localization'),
  launchLocalized:()=>invoke<LocalState>('launch_localized_copy'),
  onLocalization:(callback:(state:LocalState)=>void)=>listen<LocalState>('localization-task',event=>callback(event.payload)),
  recovery:()=>invoke<RecoveryState>('get_recovery_state'),
  restoreBackup:(entryId:string)=>invoke<LocalState>('restore_localized_backup',{entryId}),
  reconcile:()=>invoke<LocalState>('reconcile_localization'),
  useCurrent:()=>invoke<LocalState>('use_current_localization'),
  archiveEntry:(entryId:string)=>invoke<void>('archive_localization_entry',{entryId})
};
