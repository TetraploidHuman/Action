const path = require('path');
const { LanguageClient, TransportKind } = require('vscode-languageclient/node');

/** @type {LanguageClient} */
let client;

/**
 * @param {import('vscode').ExtensionContext} context
 */
function activate(context) {
    const config = require('vscode').workspace.getConfiguration('action.lsp');
    const command = config.get('path', 'action');

    const serverOptions = {
        command,
        args: ['lsp'],
        options: { env: process.env }
    };

    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'action' }],
        synchronize: {
            fileEvents: require('vscode').workspace.createFileSystemWatcher('**/*.at')
        }
    };

    client = new LanguageClient(
        'action-lsp',
        'Action Language Server',
        serverOptions,
        clientOptions
    );

    context.subscriptions.push(client.start());
}

function deactivate() {
    if (client) {
        return client.stop();
    }
}

module.exports = { activate, deactivate };
