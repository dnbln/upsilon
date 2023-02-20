// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const lightCodeTheme = require('prism-react-renderer/themes/github');
const darkCodeTheme = require('prism-react-renderer/themes/dracula');

const docsBase = 'https://github.com/dnbln/upsilon/blob/trunk/docs/';

/** @type {import('@docusaurus/types').Config} */
const config = {
    title: 'Upsilon Docs',
    tagline: 'Upsilon Docs',
    favicon: 'img/favicon.png',

    // Set the production url of your site here
    url: 'https://upsilon-docs.dnbln.dev',
    // Set the /<baseUrl>/ pathname under which your site is served
    // For GitHub pages deployment, it is often '/<projectName>/'
    baseUrl: '/',

    // GitHub pages deployment config.
    // If you aren't using GitHub pages, you don't need these.
    organizationName: 'dnbln', // Usually your GitHub org/user name.
    projectName: 'upsilon', // Usually your repo name.

    onBrokenLinks: 'throw',
    onBrokenMarkdownLinks: 'warn',

    // Even if you don't use internalization, you can use this field to set useful
    // metadata like html lang. For example, if your site is Chinese, you may want
    // to replace "en" with "zh-Hans".
    i18n: {
        defaultLocale: 'en',
        locales: ['en'],
    },

    presets: [
        [
            'classic',
            /** @type {import('@docusaurus/preset-classic').Options} */
            ({
                pages: {
                    routeBasePath: '/',
                },
                docs: false,
                blog: {
                    showReadingTime: true,
                    // Please change this to your repo.
                    // Remove this to remove the "edit this page" links.
                    editUrl: docsBase,
                },
                theme: {
                    customCss: require.resolve('./src/css/custom.css'),
                },
            }),
        ],
    ],

    plugins: [
        [
            '@docusaurus/plugin-content-docs',
            /** @type {import("@docusaurus/plugin-content-docs").Options} */
            ({
                id: 'tutorial',
                path: './tutorial',
                routeBasePath: '/tutorial',
                sidebarPath: require.resolve('./sidebars.js'),
                // Please change this to your repo.
                // Remove this to remove the "edit this page" links.
                editUrl: docsBase,
                showLastUpdateAuthor: true,
                showLastUpdateTime: true,
            }),
        ],
        [
            '@docusaurus/plugin-content-docs',
            /** @type {import("@docusaurus/plugin-content-docs").Options} */
            ({
                id: 'contributor_guide',
                path: './contributor-guide',
                routeBasePath: '/contributor-guide',
                sidebarPath: require.resolve('./sidebars.js'),
                // Please change this to your repo.
                // Remove this to remove the "edit this page" links.
                editUrl: docsBase,
                showLastUpdateAuthor: true,
                showLastUpdateTime: true,
            }),
        ],
    ],

    themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
        ({
            // Replace with your project's social card
            image: 'img/upsilon.png',
            navbar: {
                title: 'Upsilon Docs',
                logo: {
                    alt: 'My Site Logo',
                    src: 'img/upsilon-transparent-white.png',
                },
                items: [
                    {
                        type: 'doc',
                        docsPluginId: 'tutorial',
                        docId: 'intro',
                        position: 'left',
                        label: 'Tutorial',
                    },
                    {to: '/blog', label: 'Blog', position: 'left'},
                    {
                        type: 'doc',
                        docsPluginId: 'contributor_guide',
                        docId: 'intro',
                        position: 'left',
                        label: 'Contributor Guide',
                    },
                    {
                        href: 'https://github.com/dnbln/upsilon',
                        label: 'GitHub',
                        position: 'right',
                    },
                ],
            },
            footer: {
                style: 'dark',
                links: [
                    {
                        title: 'Tutorial',
                        items: [
                            {
                                label: 'Tutorial',
                                to: '/tutorial/intro',
                            },
                        ],
                    },
                    {
                        title: 'More',
                        items: [
                            {
                                label: 'Blog',
                                to: '/blog',
                            },
                            {
                                label: 'Contributor Guide',
                                to: '/contributor-guide/intro',
                            },
                            {
                                label: 'GitHub',
                                href: 'https://github.com/dnbln/upsilon',
                            },
                        ],
                    },
                ],
                copyright: `Copyright © ${new Date().getFullYear()} Dinu Blanovschi. Built with Docusaurus.`,
            },
            prism: {
                theme: lightCodeTheme,
                darkTheme: darkCodeTheme,
                additionalLanguages: ['rust', 'toml'],
            },
        }),
};

module.exports = config;
