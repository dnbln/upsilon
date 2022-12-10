cd book

echo upsilon-docs.dnbln.dev > ./CNAME

copy nul .nojekyll

git init
git add .

git commit -m "Deploy docs"

git branch gh-pages
git remote add origin https://github.com/dnbln/upsilon
git push -f origin gh-pages