const {app, BrowserWindow, Menu, ipcMain, dialog} = require('electron');
const path = require('path');
const fs = require('fs');

app.disableHardwareAcceleration();

function createWindow() {
  const win = new BrowserWindow({
    width: 1200, height: 800, minWidth: 800, minHeight: 500,
    frame: false, center: true,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true, nodeIntegration: false,
    },
  });

  Menu.setApplicationMenu(null);
  win.loadFile(path.join(__dirname, '..', 'www', 'index.html'));

  ipcMain.handle('winctl', (_, a) => {
    if (a === 'min') win.minimize();
    else if (a === 'max') { win.isMaximized() ? win.unmaximize() : win.maximize(); }
    else if (a === 'close') win.close();
  });

  ipcMain.handle('readFile', async (_, p) => {
    try { return await fs.promises.readFile(p, 'utf-8'); }
    catch (e) { return null; }
  });

  ipcMain.handle('writeFile', async (_, p, c) => {
    try { await fs.promises.writeFile(p, c, 'utf-8'); return true; }
    catch (e) { return false; }
  });

  ipcMain.handle('dialogOpen', async (_, opts) => {
    const result = await dialog.showOpenDialog(win, {
      properties: ['openFile', 'multiSelections'],
      filters: opts && opts.filters ? opts.filters : [
        {name: 'Markdown', extensions: ['md']},
        {name: 'Text Files', extensions: ['txt','log','ini','cfg','conf','yaml','yml','json','xml','csv','html','css','js','ts','py','rb','sh','bat','ps1']},
        {name: 'All Files', extensions: ['*']}
      ]
    });
    return result.canceled ? null : result.filePaths;
  });

  ipcMain.handle('dialogSave', async (_, opts) => {
    const result = await dialog.showSaveDialog(win, {
      defaultPath: opts && opts.defaultPath ? opts.defaultPath : 'untitled.md',
      filters: opts && opts.filters ? opts.filters : [
        {name: 'Markdown', extensions: ['md']},
        {name: 'Text Files', extensions: ['txt','log','ini','cfg','conf','yaml','yml','json','xml','csv','html','css','js','ts','py','rb','sh','bat','ps1']},
        {name: 'All Files', extensions: ['*']}
      ]
    });
    return result.canceled ? null : result.filePath;
  });
}

app.whenReady().then(createWindow);
app.on('window-all-closed', () => app.quit());
app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) createWindow();
});
