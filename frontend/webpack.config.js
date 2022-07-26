"use strict";

const { CleanWebpackPlugin } = require("clean-webpack-plugin");
const ForkTsCheckerWebpackPlugin = require("fork-ts-checker-webpack-plugin");
const CopyPlugin = require("copy-webpack-plugin");
const path = require("path");
const { APP_PATH, OUT_PATH, STATIC_PATH } = require("./constants");

module.exports = (_env, argv) => ({
    entry: APP_PATH,
    context: __dirname,

    output: {
        filename: "[name].bundle.js",
        path: OUT_PATH,
        publicPath: "/~assets/",
    },
    optimization: {
        // This disables the automatic chunk splitting by webpack. This is only
        // temporary until we use proper code splitting. But for now we only
        // have a few dynamic imports to split certain things manually.
        splitChunks: {
            chunks: () => false,
        },
    },

    resolve: {
        extensions: [".ts", ".tsx", ".js", ".json"],
    },

    module: {
        rules: [{
            test: /\.[jt]sx?$/u,
            loader: "babel-loader",
            include: [
                APP_PATH,
                ...argv.mode === "development"
                    ? []
                    : [path.join(__dirname, "node_modules")],
            ],
        }, {
            test: /\.yaml$/u,
            loader: "yaml-loader",
        }, {
            test: /\.svg$/u,
            use: [{
                loader: "@svgr/webpack",
                options: {
                    icon: true,
                },
            }],
        }, {
            test: /\.css$/u,
            type: "asset/source",
        }],
    },

    plugins: [
        new CleanWebpackPlugin(),
        new ForkTsCheckerWebpackPlugin({
            eslint: {
                files: ["."],
            },
            typescript: {
                mode: "write-references",
            },
            formatter: "basic",
        }),
        new CopyPlugin({
            patterns: [
                { from: path.join(APP_PATH, "index.html"), to: path.join(OUT_PATH) },
                { from: path.join(APP_PATH, "fonts.css"), to: path.join(OUT_PATH) },
                { from: STATIC_PATH, to: OUT_PATH },
            ],
        }),
    ],

    devtool: "hidden-source-map",
});
