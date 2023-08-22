# Simple RSS feed aggregator

This is a RSS feed aggregator aimed at making it easier to combine a bunch RSS feeds from different online profiles (mastodon, lemmy, pixelfed, opengameart, itch.io) into a single unified feed. The end goal of this project is to allow for stupidly simple [PESOS](https://indieweb.org/PESOS) (Publish Elsewhere, Syndicate (to your) Own Site) from platforms that support RSS feeds.

### Usage

```bash
# this stores all of the given feeds in a local JSON file `db.json`
./syndication_junction fetch https://marmadilemanteater.dev/blog/rss.xml https://gamemaking.social/@emma.rss https://programming.dev/feeds/u/emma.xml?sort=New https://opengameart.org/users/105608/art.xml https://itch.io/games/newest/by-marmadilemanteater.xml https://pxlmo.com/users/emma.atom

# this outputs an RSS to a feed named `rss.xml`
./syndication_junction output-rss

# this 
# - outputs an RSS feed to a feed name `whatever-happened-to-rss.xml`
# - downloads all of the images in the feed to a local file named `media/`
# - replaces all links to images with links that start with `https://maramdilemanteater.dev/feed/media/`
./syndication_junction output-rss whatever-happened-rss.xml https://maramdilemanteater.dev/feed
# if you then take the output `media/` directory and put it in a place where it can be accessed from that URI,
# you have a complete feed without having to rely on external media files
```

There are also more configuration options stored in `db.json`. These are more-so things you might not want to change very frequently such as:
 - The title attached to the feed
   ```jsonc
   {
     /* ... */
     "title": "title goes here",
     /* ... */
   }
   ```
 - The link attached to the feed
   ```jsonc
   {
     /* ... */
     "link": "link goes here",
     /* ... */
   }
 - The amount of posts a feed contains (all posts are stored in db.json for historical purposes, this just changes the output feed)
   ```jsonc
   {
     /* ... */
     "max_entries_published": -1,/* -1 is all posts */
     /* ... */
   }
   ```
  - Whether or not to fill in empty titles _(this is important for things like mastodon which leave title empty and make RSS readers look funny)_
     ```jsonc
    {
      /* ... */
      "include_description_as_title_if_none_given": true,
      /* ... */
    }
    ```
  - How many words from the description to use as auto-title
     ```jsonc
    {
      /* ... */
      "description_title_word_count": 10,
      /* ... */
    }
    ```
  - What to use as the ellipsis after a post with an auto-title
     ```jsonc
    {
      /* ... */
      "title_ellipsis": "...",
      /* ... */
    }
    ```
  - Whether or not to add a `content:encoded` element with the contents of the `description` element
     ```jsonc
    {
      /* ... */
      "populate_content_encoded": true,
      /* ... */
    }
    ```
  - Whether or not to add attached media items to the `content:encoded` element _(Some RSS readers don't show attached media items on posts, so this is a way of including images in a way that is visible to more RSS readers)_
     ```jsonc
    {
      /* ... */
      "add_media_to_content_encoded": true,
      /* ... */
    }
    ```
  - Whether or not to override entry authors with the feed's author _(useful for pixelfed since all posts have an author field on them)_
     ```jsonc
    {
      /* ... */
      "override_item_author": true
      /* ... */
    }
    ```

There are also configuration options per feed inside of the `rss` property of `db.json`:

  ```jsonc
  {
    "rss": {
      "https://marmadilemanteater.dev/blog/rss.xml": {
        // the RSS data saved in this file from the feed
        "rss": { /* ... */ },
        // a command which syndication_junction will pipe the raw feed into and which is expected to output a slightly modified version of the feed
        "manipulate_input": "",// EX: `sed 's/something/some other thing/'`
        // currently unused atm; please, ignore
        "retain_all_entries": true,
        // override title of feed
        "title": "Emma",
        // override link to feed
        "link": "https://marmadilemanteater.dev/blog/"
      },
      /* ... */
    }
    /* ... */
  }
  ```
