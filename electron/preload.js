const {contextBridge, ipcRenderer} = require('electron');

contextBridge.exposeInMainWorld('__ELECTRON__', {
  winctl: a => ipcRenderer.invoke('winctl', a),
  startDrag: () => ipcRenderer.invoke('startDrag'),
  readFile: p => ipcRenderer.invoke('readFile', p),
  writeFile: (p, c) => ipcRenderer.invoke('writeFile', p, c),
  dialogOpen: opts => ipcRenderer.invoke('dialogOpen', opts),
  dialogSave: opts => ipcRenderer.invoke('dialogSave', opts),
  platform: process.platform,
});
